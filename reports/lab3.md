# 功能实现

在ch5的实验中，我做了如下工作：
1. 实现spawn方法：结合fork+exec的流程，但不对父进程的memory_set等进行复制；
2. 实现stride 调度算法，并通过一个溢出检测+重新置位的机制，在保证算法正确的同时，妥善处理了溢出问题。

# 问答题

A1：不是，因为8bit的最大值是255，250+10=260，会溢出得到4，然后根据算法流程，4比250小，依然会调度p2，且接下来很多次都会调度p2，从而导致调度十分不平衡。

A2-1：用归纳法，初始时所有进程stride间距离都不超过BIGSTRIDE/2（为0），而对于任意一步，由于最小stride最多只加BIGSTRIDE/2，则该条件依然满足，证毕。

A2-2：
```rust
impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let a = (self.0 & 0xff) as u16;
        let b = (other.0 & 0xff) as u16;
        if (a > b && a < b + 255 / 2) || (a < b && a + 255 / 2) < b {
            Some(Ordering::Greater)
        } else {
            Some(Ordering::Less)
        }
    }
}
```
# 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我未与他人就（与本次实验相关的）方面做过交流。

2. 此外，我参考了以下资料：
[RUST圣经](https://course.rs/about-book.html) 
[rCore-Tutorial-Guide 2024 秋季学期文档](https://learningos.cn/rCore-Camp-Guide-2024A/) 

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。
