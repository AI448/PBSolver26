use std::fmt::Debug;

use crate::Comparator;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Color {
    BLACK,
    RED,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Direction {
    LEFT = 0,
    RIGHT = 1,
}

impl std::ops::Neg for Direction {
    type Output = Self;
    #[inline(always)]
    fn neg(self) -> Self::Output {
        [Direction::RIGHT, Direction::LEFT][self as usize]
    }
}

impl<T> std::ops::Index<Direction> for [T; 2] {
    type Output = T;
    #[inline(always)]
    fn index(&self, index: Direction) -> &Self::Output {
        &self[index as usize]
    }
}

impl<T> std::ops::IndexMut<Direction> for [T; 2] {
    #[inline(always)]
    fn index_mut(&mut self, index: Direction) -> &mut Self::Output {
        &mut self[index as usize]
    }
}

#[derive(Clone, Debug)]
struct Node {
    children: [usize; 2],
    parent: usize,
    direction_as_child: Direction,
    color: Color,
}

/// 常にソートされたマップ
#[derive(Clone)]
pub struct SortedMap<ValueT, CompareT> {
    comparator: CompareT,
    position_array: Vec<usize>,
    item_array: Vec<(usize, ValueT)>,
    node_array: Vec<Node>,
    root: usize,
}

impl<ValueT, CompareT> Default for SortedMap<ValueT, CompareT>
where
    CompareT: Default,
{
    #[inline(always)]
    fn default() -> Self {
        Self {
            comparator: CompareT::default(),
            position_array: Vec::default(),
            item_array: Vec::default(),
            node_array: Vec::default(),
            root: Self::NULL,
        }
    }
}

impl<ValueT, CompareT> SortedMap<ValueT, CompareT> {
    const NULL: usize = usize::MAX;
}

impl<ValueT, CompareT> SortedMap<ValueT, CompareT>
where
    CompareT: Comparator<(usize, ValueT)>,
{
    #[inline(always)]
    pub fn new(compare: CompareT) -> Self {
        Self {
            comparator: compare,
            position_array: Vec::default(),
            item_array: Vec::default(),
            node_array: Vec::default(),
            root: Self::NULL,
        }
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.node_array.is_empty()
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.node_array.len()
    }

    #[inline(always)]
    pub fn min_key_value(&self) -> Option<&(usize, ValueT)> {
        let node = self.search_end_descendant(self.root, Direction::LEFT);
        if node == Self::NULL {
            None
        } else {
            Some(&self.item_array[node])
        }
    }

    #[inline(always)]
    pub fn max_key_value(&self) -> Option<&(usize, ValueT)> {
        let node = self.search_end_descendant(self.root, Direction::RIGHT);
        if node == Self::NULL {
            None
        } else {
            Some(&self.item_array[node])
        }
    }

    #[inline(always)]
    pub fn contains_key(&self, index: usize) -> bool {
        self.position_array[index] != Self::NULL
    }

    #[inline(always)]
    pub fn get(&self, index: usize) -> Option<&ValueT> {
        if let Some(&id) = self.position_array.get(index)
            && id != Self::NULL
        {
            return Some(&self.item_array[id].1);
        } else {
            return None;
        }
    }

    #[inline(never)]
    pub fn insert(&mut self, index: usize, value: ValueT) {
        #[cfg(test)]
        self.check_consistency(self.root);
        let new_item = (index, value);
        if index >= self.position_array.len() {
            self.position_array
                .resize(1usize << index.bit_width(), Self::NULL);
        }

        if self.position_array[index] != Self::NULL {
            // すでに index が存在するなら削除
            let current = self.position_array[index];
            debug_assert!(self.item_array[current].0 == index);
            if self.comparator.le(&self.item_array[current], &new_item) {
                // 元の要素よりも大きくなった場合
                // current よりも大きい最小の要素を探索
                let next = self.search_next(current, Direction::RIGHT);
                if next == Self::NULL || self.comparator.le(&new_item, &self.item_array[next]) {
                    // 変更後の要素が next よりも小さいならソートし直す必要はない
                    self.item_array[current].1 = new_item.1;
                    return;
                }
            } else {
                // もとの要素よりも大きくなっていない場合
                // current よりも小さい最大の要素を探索
                let next = self.search_next(current, Direction::LEFT);
                if next == Self::NULL || self.comparator.le(&self.item_array[next], &new_item) {
                    // 変更後の要素が next よりも小さいならソートし直す必要はない
                    self.item_array[current].1 = new_item.1;
                    return;
                }
            }
            // ソートし直す必要がある場合には一度削除する
            self.remove(index);
        }
        // 新たに追加するノード
        let current = self.node_array.len();
        // 挿入する親ノードを探索
        let (parent, direction) = self.search_inserting_leaf(&new_item);
        // 親に枝を追加
        if parent == Self::NULL {
            debug_assert!(current == 0);
            self.root = current;
        } else {
            debug_assert!(self.node_array[parent].children[direction] == Self::NULL);
            self.node_array[parent].children[direction] = current;
        }
        // アイテム・ノードを追加
        self.position_array[index] = current;
        self.item_array.push(new_item);
        self.node_array.push(Node {
            parent: parent,
            direction_as_child: direction,
            children: [Self::NULL, Self::NULL],
            color: Color::RED,
        });
        // 挿入の後処理
        self.rebalance_after_insertion(current);

        #[cfg(test)]
        self.check_consistency(self.root);
    }

    /// sorted_map が index を含んでいれば index を削除してその値を返す
    #[inline(never)]
    pub fn remove(&mut self, index: usize) -> Option<ValueT> {
        #[cfg(test)]
        self.check_consistency(self.root);

        // index に対応する要素が存在しない場合には None を返す
        if index >= self.position_array.len() || self.position_array[index] == Self::NULL {
            return None;
        }

        // 削除するノードの位置を取得(※この処理ではアイテムの位置が変わることがある点に注意)
        let removing = {
            // index に対応するノードの位置
            let current = self.position_array[index];
            if self.node_array[current].children[Direction::LEFT] != Self::NULL
                && self.node_array[current].children[Direction::RIGHT] != Self::NULL
            {
                // current が左右両方の子を持つ場合
                // current の右側で最も小さいノードを探索
                let next = self.search_end_descendant(
                    self.node_array[current].children[Direction::RIGHT],
                    Direction::LEFT,
                );
                debug_assert!(self.node_array[next].children[Direction::LEFT] == Self::NULL);
                // current と next のアイテムを入れ替え
                let next_index = self.item_array[next].0;
                debug_assert!(self.position_array[next_index] == next);
                self.position_array.swap(index, next_index);
                self.item_array.swap(current, next);
                // next が削除対象
                next
            } else {
                // たかだか 1 つの子を持つ場合
                // current が削除対象
                current
            }
        };
        debug_assert!(removing != Self::NULL);
        debug_assert!(self.item_array[removing].0 == index);
        debug_assert!(
            self.node_array[removing]
                .children
                .iter()
                .any(|c| *c == Self::NULL)
        );

        // ノードを削除(※これ以降 remving に触らないこと)
        // parent の direction 側の子が，元々 remving が存在していた場所になる
        let (removed_index, removed_value, parent, direction, removed_color) =
            self.remove_node(removing);
        debug_assert!(removed_index == index);
        if removed_color == Color::BLACK {
            // 削除されたノードがルートノード以外の黒ノードであった場合にはリバランス
            self.rebalance_after_removing(parent, direction);
        }

        #[cfg(test)]
        self.check_consistency(self.root);

        return Some(removed_value);
    }

    #[inline(never)]
    pub fn clear(&mut self) {
        for (index, _) in self.item_array.iter() {
            debug_assert!(self.position_array[*index] != Self::NULL);
            self.position_array[*index] = Self::NULL;
        }
        self.position_array.clear();
        self.item_array.clear();
        self.node_array.clear();
        self.root = Self::NULL;
    }

    #[inline(always)]
    pub fn iter(&self) -> impl std::iter::Iterator<Item = (&usize, &ValueT)> + Clone {
        self.item_array.iter().map(|(index, value)| (index, value))
    }

    #[inline(always)]
    pub fn iter_in_ascending_order(
        &self,
    ) -> impl std::iter::DoubleEndedIterator<Item = (&usize, &ValueT)> {
        Iterator::new(self)
    }

    #[inline(always)]
    pub fn iter_in_descending_order(
        &self,
    ) -> impl std::iter::DoubleEndedIterator<Item = (&usize, &ValueT)> {
        Iterator::new(self).rev()
    }

    /// index, value を挿入すべきノードを探索する
    fn search_inserting_leaf(&self, new_item: &(usize, ValueT)) -> (usize, Direction) {
        if self.root == Self::NULL {
            return (Self::NULL, Direction::LEFT);
        } else {
            // リーフに向かって再帰
            let mut current = self.root;
            loop {
                let item = &self.item_array[current];
                if self.comparator.le(&new_item, &item) {
                    let left_child = self.node_array[current].children[Direction::LEFT];
                    if left_child == Self::NULL {
                        return (current, Direction::LEFT);
                    } else {
                        current = left_child;
                    }
                } else {
                    let right_child = self.node_array[current].children[Direction::RIGHT];
                    if right_child == Self::NULL {
                        return (current, Direction::RIGHT);
                    } else {
                        current = right_child;
                    }
                }
            }
        }
    }

    /// insert 後に赤制約を満たすように木を修正する insert のサブルーチン
    fn rebalance_after_insertion(&mut self, node_id: usize) {
        let mut current = node_id;
        loop {
            debug_assert!(self.node_array[current].color == Color::RED);
            if current == self.root {
                // 現在のノードがルートである場合
                // 黒に変更して終了
                self.node_array[current].color = Color::BLACK;
                return;
            } else {
                // 親ノード
                let parent = self.node_array[current].parent;
                if self.node_array[parent].color == Color::BLACK {
                    // 親が黒である場合
                    // 条件が満たされたので終了
                    return;
                } else {
                    // 親が赤である場合
                    // 祖父ノード(親が赤なので必ず存在し黒)
                    let grandparent = self.node_array[parent].parent;
                    debug_assert!(self.node_array[grandparent].color == Color::BLACK);
                    // 祖父ノードから見た親ノードの向き
                    let parent_direction = self.node_array[parent].direction_as_child;
                    debug_assert!(
                        self.node_array[grandparent].children[parent_direction] == parent
                    );
                    // 叔父ノード(リーフである可能性がある)
                    let uncle = self.node_array[grandparent].children[-parent_direction];
                    if uncle != Self::NULL && self.node_array[uncle].color == Color::RED {
                        // 叔父ノードが赤である場合
                        // 祖父・親・叔父ノードの色を変更
                        self.node_array[grandparent].color = Color::RED;
                        self.node_array[parent].color = Color::BLACK;
                        self.node_array[uncle].color = Color::BLACK;
                        // 祖父ノードに移動して継続
                        current = grandparent;
                        continue;
                    } else {
                        // 叔父ノードが黒
                        // 親ノードから見た現在のノードの向き
                        let direction = self.node_array[current].direction_as_child;
                        debug_assert!(self.node_array[parent].children[direction] == current);
                        if direction == parent_direction {
                            // 子の向きと親の向きが同一である場合
                            // 親ノードの色を変更
                            self.node_array[parent].color = Color::BLACK;
                        } else {
                            // 子の向きと親の向きが異なっている場合
                            // 親ノードを回転
                            self.rotate(parent, parent_direction);
                            // 現在のノード(親になった)の色を変更
                            self.node_array[current].color = Color::BLACK;
                        }
                        // 祖父ノードの色を変更
                        self.node_array[grandparent].color = Color::RED;
                        // 祖父ノードを回転(親ノードが上になるように)して終了
                        self.rotate(grandparent, -parent_direction);
                        return;
                    }
                }
            }
        }
    }

    /// node を削除し，index, value と node の元親ノードの位置, 元親ノードから見た node の向き, 削除したノードの色を返す
    /// node の子は 1 つ以下であること(削除時に親と子を接続するため)
    /// この関数の呼び出しによってノードの位置が変わるため，呼び出し前に取得したノードの位置を呼び出し後に使用しないこと
    fn remove_node(&mut self, node: usize) -> (usize, ValueT, usize, Direction, Color) {
        debug_assert!(node != Self::NULL);
        debug_assert!(
            self.node_array[node].children[Direction::LEFT] == Self::NULL
                || self.node_array[node].children[Direction::RIGHT] == Self::NULL
        );
        let removed_item;
        let removed_node;
        if node + 1 == self.node_array.len() {
            removed_item = self.item_array.pop().unwrap();
            removed_node = self.node_array.pop().unwrap();
        } else {
            // 末尾要素を node に移動し，移動したノードへの接続を修復する
            removed_item = self.item_array.swap_remove(node);
            removed_node = self.node_array.swap_remove(node);
            // 移動前の位置(debug_assert でしか使わない)
            let previous_node = self.node_array.len();
            // self.position_array からの接続を修復
            let index = self.item_array[node].0;
            debug_assert!(self.position_array[index] == previous_node);
            self.position_array[index] = node;
            // 親ノードからの接続を修復
            let parent = self.node_array[node].parent;
            let direction_as_child = self.node_array[node].direction_as_child;
            if parent == Self::NULL {
                debug_assert!(self.root == previous_node);
                self.root = node;
            } else if parent != node {
                debug_assert!(
                    self.node_array[parent].children[direction_as_child] == previous_node
                );
                self.node_array[parent].children[direction_as_child] = node;
            }
            // 子ノードからの接続を修復
            for direction in [Direction::LEFT, Direction::RIGHT] {
                let child = self.node_array[node].children[direction];
                if child != Self::NULL && child != node {
                    debug_assert!(self.node_array[child].parent == previous_node);
                    debug_assert!(self.node_array[child].direction_as_child == direction);
                    self.node_array[child].parent = node;
                }
            }
        }
        // position_array から削除したノードへの接続を解消
        debug_assert!(self.position_array[removed_item.0] == node);
        self.position_array[removed_item.0] = Self::NULL;
        // 削除したノードの元親ノード
        let parent = {
            if removed_node.parent != self.node_array.len() {
                removed_node.parent
            } else {
                node
            }
        };
        let direction_as_child = removed_node.direction_as_child;
        // 削除したノードの元子ノード
        let child = {
            let child = {
                if removed_node.children[Direction::LEFT] != Self::NULL {
                    removed_node.children[Direction::LEFT]
                } else {
                    removed_node.children[Direction::RIGHT]
                }
            };
            if child != self.node_array.len() {
                child
            } else {
                node
            }
        };
        // 元親ノードと元子ノードを接続
        if parent == Self::NULL {
            debug_assert!(self.root == node);
            self.root = child;
        } else {
            debug_assert!(self.node_array[parent].children[direction_as_child] == node);
            self.node_array[parent].children[direction_as_child] = child;
        }
        if child != Self::NULL {
            debug_assert!(self.node_array[child].parent == node);
            self.node_array[child].parent = parent;
            self.node_array[child].direction_as_child = direction_as_child;
        }
        return (
            removed_item.0,
            removed_item.1,
            parent,
            direction_as_child,
            removed_node.color,
        );
    }

    /// node から見て direction 側の黒ノードの深さが 1 浅い状態なので，左右の黒ノードの深さを揃える
    fn rebalance_after_removing(&mut self, node: usize, direction: Direction) {
        // 現在注目している部分木の親ノード
        let mut parent = node;
        // 黒深さが足りていない子の向き
        let mut direction = direction;

        // 初回のみのチェック
        if parent == Self::NULL {
            // ルートノードが削除された場合
            if self.root != Self::NULL {
                // 新たなルートノードが存在すれば色を黒にする
                debug_assert!(self.node_array.len() == 1);
                debug_assert!(self.node_array[self.root].color == Color::RED);
                self.node_array[self.root].color = Color::BLACK;
            } else {
                // 空になった場合には何もしない
                debug_assert!(self.node_array.len() == 0);
            }
            return;
        } else {
            // parent の子ノードで黒深さが小さい方(NULL になり得る)
            let current = self.node_array[parent].children[direction];
            if current != Self::NULL && self.node_array[current].color == Color::RED {
                // 黒にして終了
                self.node_array[current].color = Color::BLACK;
                return;
            }
        }

        // 黒深さが揃うまで反復
        loop {
            // parent の子ノードで黒深さが小さい方(NULL または黒)
            let current = self.node_array[parent].children[direction];
            debug_assert!(current == Self::NULL || self.node_array[current].color == Color::BLACK);
            // parent の子ノードで黒深さが大きい方
            let brother = self.node_array[parent].children[-direction];
            // current に近い方の甥
            let near_nephew = self.node_array[brother].children[direction];
            // current から遠い方の甥
            let far_nephew = self.node_array[brother].children[-direction];

            #[cfg(test)]
            assert!(self.check_consistency(current) + 1 == self.check_consistency(brother));

            if self.node_array[brother].color == Color::BLACK {
                // 兄弟が黒
                if far_nephew == Self::NULL || self.node_array[far_nephew].color == Color::BLACK {
                    // 遠い方の甥が存在しないまたは黒
                    if near_nephew == Self::NULL
                        || self.node_array[near_nephew].color == Color::BLACK
                    {
                        // 近い方の甥が存在しないまたは黒
                        if self.node_array[parent].color == Color::BLACK {
                            // みんな黒
                            // 兄弟を赤に(これにより parent ではバランスする)
                            self.node_array[brother].color = Color::RED;
                            if parent == self.root {
                                // parent がルートであれば終了
                                return;
                            } else {
                                // parent の黒深さが 1 足りない状態なので上に再帰
                                direction = self.node_array[parent].direction_as_child;
                                parent = self.node_array[parent].parent;
                                continue;
                            }
                        } else {
                            // 親だけが赤
                            self.node_array[parent].color = Color::BLACK;
                            self.node_array[brother].color = Color::RED;
                            return;
                        }
                    } else {
                        // 近い方の甥が存在して赤かつ，遠い方の甥が存在しないまたは黒
                        debug_assert!(self.node_array[brother].color == Color::BLACK);
                        self.rotate(brother, -direction);
                        self.rotate(parent, direction);
                        self.node_array[near_nephew].color = self.node_array[parent].color;
                        self.node_array[brother].color = Color::BLACK;
                        self.node_array[parent].color = Color::BLACK;
                        return;
                    }
                } else {
                    // 遠い方の甥が存在して赤（近い方の甥の色は任意）
                    debug_assert!(self.node_array[brother].color == Color::BLACK);
                    self.rotate(parent, direction);
                    self.node_array[brother].color = self.node_array[parent].color;
                    self.node_array[parent].color = Color::BLACK;
                    self.node_array[far_nephew].color = Color::BLACK;
                    return;
                }
            } else {
                // 兄弟が赤
                debug_assert!(self.node_array[parent].color == Color::BLACK);
                debug_assert!(near_nephew != Self::NULL);
                debug_assert!(self.node_array[near_nephew].color == Color::BLACK);
                debug_assert!(far_nephew != Self::NULL);
                debug_assert!(self.node_array[far_nephew].color == Color::BLACK);
                // brother を親に
                self.rotate(parent, direction);
                self.node_array[parent].color = Color::RED;
                self.node_array[brother].color = Color::BLACK;
                // NOTE: parent, direction の更新は不要で次の反復で終了する
                continue;
            }
        }
    }

    /// node を direction 方向に回転する
    fn rotate(&mut self, node: usize, direction: Direction) {
        debug_assert!(node != Self::NULL);
        debug_assert!(direction == Direction::LEFT || direction == Direction::RIGHT);
        // 注目しているノード
        let current = node;
        // parent から見た current の向き
        let current_direction = self.node_array[current].direction_as_child;
        // 回転後に current に代わって親になるノード
        let next = self.node_array[current].children[-direction];
        // current の親ノード
        let parent = self.node_array[current].parent;
        // next の子から current の子になるノード
        let child = self.node_array[next].children[direction];
        // parent の子を next に
        if parent == Self::NULL {
            debug_assert!(current == self.root);
            self.root = next;
        } else {
            debug_assert!(self.node_array[parent].children[current_direction] == current);
            self.node_array[parent].children[current_direction] = next;
        }
        // next の親を parent に
        self.node_array[next].parent = parent;
        self.node_array[next].direction_as_child = current_direction;
        // next の direction 側の子を current に
        self.node_array[next].children[direction] = current;
        // current の親を next に
        self.node_array[current].parent = next;
        self.node_array[current].direction_as_child = direction;
        // current の direction の逆側の子を child に
        self.node_array[current].children[-direction] = child;
        // child の親を current に
        if child != Self::NULL {
            debug_assert!(self.node_array[child].parent == next);
            self.node_array[child].parent = current;
            self.node_array[child].direction_as_child = -direction;
        }
    }

    /// node の次に小さいまたは大きい要素を探索
    /// node が NULL の場合には最小または最大要素を返す
    #[inline(always)]
    fn search_next(&self, node: usize, direction: Direction) -> usize {
        if node == Self::NULL {
            return self.search_end_descendant(self.root, -direction);
        } else {
            let child = self.node_array[node].children[direction];
            if child != Self::NULL {
                return self.search_end_descendant(child, -direction);
            } else {
                return self.search_first_ancestor(node, direction);
            }
        }
    }

    /// nodeの最小または最大の子孫を探索
    /// node が NULL であれば NULL を返す
    #[inline(always)]
    fn search_end_descendant(&self, node: usize, direction: Direction) -> usize {
        if node == Self::NULL {
            return Self::NULL;
        }
        let mut current = node;
        loop {
            let child = self.node_array[current].children[direction];
            if child == Self::NULL {
                return current;
            } else {
                current = child;
            }
        }
    }

    /// node の direction 方向の最初の祖先を探索
    /// node は NULL でないこと
    #[inline(always)]
    fn search_first_ancestor(&self, node: usize, direction: Direction) -> usize {
        debug_assert!(node != Self::NULL);
        let mut current = node;
        loop {
            let parent = self.node_array[current].parent;
            let direction_as_child = self.node_array[current].direction_as_child;
            if parent == Self::NULL {
                return Self::NULL;
            } else if -direction_as_child == direction {
                return parent;
            } else {
                current = parent;
            }
        }
    }

    /// node 以下の部分木の整合性をチェックし，黒深さを返す
    #[cfg(test)]
    fn check_consistency(&self, node: usize) -> usize {
        if node == Self::NULL {
            return 0;
        } else {
            let color = self.node_array[node].color;
            let item = &self.item_array[node];
            // position_array が item の位置を正しく指している
            assert!(self.position_array[item.0] == node);
            // ルートである ⇔ 親が NULL
            assert!((node == self.root) == (self.node_array[node].parent == Self::NULL));
            // ルートである ⇒ 黒
            assert!(node != self.root || color == Color::BLACK);

            let left = self.node_array[node].children[Direction::LEFT];
            if left != Self::NULL {
                // 左の子の親は自身である
                assert!(self.node_array[left].parent == node);
                // 左の子は左の子であることを自認している
                assert!(self.node_array[left].direction_as_child == Direction::LEFT);
                // 親が赤であれば左の子は黒
                assert!(color == Color::BLACK || self.node_array[left].color == Color::BLACK);
                // 左の子は親以下
                let left_item = &self.item_array[left];
                assert!(self.comparator.le(&left_item, &item));
            }

            let right = self.node_array[node].children[Direction::RIGHT];
            if right != Self::NULL {
                // 右の子の親は自身である
                assert!(self.node_array[right].parent == node);
                // 右の子は右の子であることを自認している
                assert!(self.node_array[right].direction_as_child == Direction::RIGHT);
                // 親が赤であれば右の子は黒
                assert!(color == Color::BLACK || self.node_array[right].color == Color::BLACK);
                // 右の子は親以上
                let right_item = &self.item_array[right];
                assert!(self.comparator.le(&item, &right_item));
            }

            // 左右の子の整合性を再帰的にチェック
            let l = self.check_consistency(left);
            let r = self.check_consistency(right);

            // 左右の黒深さは等しい
            assert!(l == r);

            return l + if color == Color::RED { 0 } else { 1 };
        }
    }
}

pub struct Iterator<'a, ValueT, CompareT> {
    sorted_map: &'a SortedMap<ValueT, CompareT>,
    current: usize,
}

impl<'a, ValueT, CompareT> Iterator<'a, ValueT, CompareT> {
    const NULL: usize = <SortedMap<ValueT, CompareT>>::NULL;

    #[inline(always)]
    fn new(sorted_map: &'a SortedMap<ValueT, CompareT>) -> Self {
        Self {
            sorted_map: sorted_map,
            current: Self::NULL,
        }
    }
}

impl<'a, ValueT, CompareT> std::iter::Iterator for Iterator<'a, ValueT, CompareT>
where
    CompareT: Comparator<(usize, ValueT)>,
{
    type Item = (&'a usize, &'a ValueT);

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let next = self.sorted_map.search_next(self.current, Direction::RIGHT);
        debug_assert!(
            self.current == Self::NULL
                || next == Self::NULL
                || self.sorted_map.comparator.le(
                    &self.sorted_map.item_array[self.current],
                    &self.sorted_map.item_array[next]
                )
        );
        self.current = next;
        if self.current == Self::NULL {
            return None;
        } else {
            let item = &self.sorted_map.item_array[self.current];
            return Some((&item.0, &item.1));
        }
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.sorted_map.len()))
    }
}

impl<'a, ValueT, CompareT> std::iter::DoubleEndedIterator for Iterator<'a, ValueT, CompareT>
where
    CompareT: Comparator<(usize, ValueT)>,
{
    #[inline(always)]
    fn next_back(&mut self) -> Option<Self::Item> {
        let next = self.sorted_map.search_next(self.current, Direction::LEFT);
        debug_assert!(
            self.current == Self::NULL
                || next == Self::NULL
                || self.sorted_map.comparator.le(
                    &self.sorted_map.item_array[next],
                    &self.sorted_map.item_array[self.current]
                )
        );
        self.current = next;
        if self.current == Self::NULL {
            return None;
        } else {
            let item = &self.sorted_map.item_array[self.current];
            return Some((&item.0, &item.1));
        }
    }
}

impl<ValueT, CompareT> std::fmt::Debug for SortedMap<ValueT, CompareT>
where
    ValueT: Debug,
    CompareT: Comparator<(usize, ValueT)>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_map()
            .entries(self.iter_in_ascending_order().map(|(i, v)| (i, v)))
            .finish()
    }
}

#[cfg(test)]
mod test {
    use crate::{NaturalComparator, ValueComparator};

    use super::SortedMap;

    #[test]
    fn test1() {
        let mut m: SortedMap<usize, ValueComparator<NaturalComparator>> = SortedMap::default();

        for i in 0..50 {
            m.insert(2 * i, 20 * i);
        }
        for i in (0..50).rev() {
            m.insert(2 * i + 1, 20 * i + 10);
        }
        dbg!(&m.root);
        dbg!(&m.node_array);
        dbg!(&m);

        let v = m.remove(0);
        dbg!(v);
        dbg!(&m);

        let v = m.remove(5);
        dbg!(v);
        dbg!(&m);

        let v = m.remove(9);
        dbg!(v);
        dbg!(&m);

        m.remove(1);
        m.remove(2);
        m.remove(3);
        m.remove(4);
        m.remove(6);
        m.remove(7);
        m.remove(8);

        for i in 50..100 {
            m.remove(i);
        }
        dbg!(&m);
    }
}
