// LeetCode 1. Two Sum
// https://leetcode.com/problems/two-sum/
//
// 難易度: Easy
// 説明: 配列内の2つの数値のインデックスを見つける。それらの和がtargetになる。
//
// 解法: ハッシュマップを使った一回走査
// 時間計算量: O(n)
// 空間計算量: O(n)

use std::collections::HashMap;

pub fn two_sum(nums: Vec<i32>, target: i32) -> Vec<i32> {
    let mut map = HashMap::new();

    for (i, &num) in nums.iter().enumerate() {
        let complement = target - num;

        if let Some(&index) = map.get(&complement) {
            return vec![index as i32, i as i32];
        }

        map.insert(num, i);
    }

    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example1() {
        assert_eq!(two_sum(vec![2, 7, 11, 15], 9), vec![0, 1]);
    }

    #[test]
    fn test_example2() {
        assert_eq!(two_sum(vec![3, 2, 4], 6), vec![1, 2]);
    }

    #[test]
    fn test_example3() {
        assert_eq!(two_sum(vec![3, 3], 6), vec![0, 1]);
    }
}

// 実行結果（LeetCodeで実行後記録）:
// Runtime: - ms, faster than -%
// Memory Usage: - MB, less than -%
// 完了日: -
