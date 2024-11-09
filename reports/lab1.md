# 功能实现

在ch3的实验中，开发了一个新的系统调用sys_task_info(_ti: *mut TaskInfo)，用于获取当前进程的状态、运行时间和各系统调用执行次数信息：
1. 进程状态：
肯定是运行时的进程才能执行该调用，直接返回running。
2. 运行时间：
在进程控制块中记录该进程的首次被执行的起始时间，运行时间即为调sys_task_info的时间减去起始时间。
3. 各系统调用执行次数：
进程控制块维护一个数组，在内核syscall函数中对相应函数调用此处进行维护即可。


# 问答题

A1: 运行三个bad测例，分别访问了非法地址、非U态指令和非U态寄存器，在进入S态后，被检出非法，导致程序异常退出。

```
[kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003a4, kernel killed it.
[kernel] IllegalInstruction in application, kernel killed it.
[kernel] IllegalInstruction in application, kernel killed it.
```
SBI版本：RustSBI version 0.3.0-alpha.2, adapting to RISC-V SBI v1.0.0

A2-1：在进入 __restore 时，寄存器 a0 通常表示用户上下文结构 TrapContext 的指针，这意味着 a0 提供了操作系统可以访问并恢复用户态程序寄存器值的地址。__restore的使用场景有：从异常（trap）或中断返回用户态、在上下文切换中恢复新的用户进程。

A2-2：（1）sstatus 寄存器：保存了用户态程序的状态信息，它包含用户态与内核态的权限位、全局中断使能位等信息。恢复 sstatus 能够确保系统正确地切换回用户态并设置正确的权限。（2）sepc寄存器：保存异常发生时的程序计数器。恢复的sepc决定了用户态程序恢复后从哪一条指令开始执行。（3）sscratch寄存器：是一个暂存寄存器，用于存放用户态栈指针。恢复sscratch可以保证能正确恢复到用户态栈上。

A2-3：x2 寄存器（栈指针）在 __restore 的汇编代码中已经被特殊处理，不需要再次加载。x4 寄存器是tp寄存器，用于进程隔离，用户态和内核态是共享的，不需要的trap和restore的时候读写。

A2-4：sp 寄存器现在存储的是用户态的栈指针，sscratch 寄存器现在存储的是内核态的栈指针。

A2-5：状态切换发生在sret。sret 指令会检查 sstatus 寄存器的 SPP 位（此前的汇编命令中已经设置为用户态），并使CPU转到对应的状态，然后把spec寄存器中存储的值记载到PC（程序计数器）上，然后开始执行用户态的对应地址的指令。

A2-6：sp 寄存器现在存储的是内核态的栈指针，sscratch 寄存器现在存储的是用户态的栈指针。

A2-7：ecall中断可以触发U态到S态。__alltraps 和 __restore应该没有对应阶段。

# 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我未与他人就（与本次实验相关的）方面做过交流。

2. 此外，我参考了以下资料：
[RUST圣经](https://course.rs/about-book.html) 
[rCore-Tutorial-Guide 2024 秋季学期文档](https://learningos.cn/rCore-Camp-Guide-2024A/) 

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。