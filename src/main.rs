
use std::future::{poll_fn, Future};
use std::pin::Pin;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use tokio::runtime::Runtime;

struct CountdownFuture {
    count: u32,
}

impl CountdownFuture {
    fn new(start: u32) -> Self {
        CountdownFuture { count: start }
    }
}

impl Future for CountdownFuture {
    type Output = &'static str;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.count == 0 {
            Poll::Ready("Liftoff!")
        } else {
            println!("{}...", self.count);
            self.count -= 1;
            // Wake immediately — we're always ready to make progress
            // cx.waker().wake_by_ref();
            cx.waker().clone().wake();
            Poll::Pending
        }
    }
}

// Usage with our mini executor or tokio:
// let msg = block_on(CountdownFuture::new(5));
// prints: 5... 4... 3... 2... 1...
// msg == "Liftoff!"io
/// 手动实现的 block_on 函数：同步执行 Future 并阻塞等待结果
/// 
/// 这是 Tokio runtime 的简化版，展示 Future 执行器的工作原理
fn block_on<F: Future>(mut future: F) -> F::Output {
    // ========== 第一步：固定 Future 在栈上 ==========
    // Pin 确保 Future 不会被移动，这对于自引用类型的 Future 至关重要
    // 
    // SAFETY 安全性说明：
    // - `future` 在这之后不会被移动
    // - 我们只通过 Pin 引用访问它，直到它完成
    // - 栈上固定是安全的，因为我们保证 future 在整个函数生命周期内有效
    let mut future = unsafe { Pin::new_unchecked(&mut future) };

    // ========== 第二步：创建一个"空操作"的 Waker ==========
    // Waker 的作用：当 Future 返回 Pending 后，条件满足时通知执行器重新 poll
    // 
    // 这里创建的是一个"空操作" Waker —— 它不做任何事
    // 因为我们使用忙循环策略，会不断轮询，不需要真正等待唤醒
    // 真正的执行器（如 Tokio）会在这里实现线程挂起/唤醒机制
    fn noop_raw_waker() -> RawWaker {
        // Waker 的四个回调函数：
        // 1. clone: 克隆 Waker
        // 2. wake: 唤醒并消费 Waker
        // 3. wake_by_ref: 唤醒但不消费 Waker  
        // 4. drop: 释放 Waker
        
        // 空操作函数：什么都不做
        fn no_op(_: *const ()) {}
        
        // clone 函数：返回一个新的空 Waker
        fn clone(_: *const ()) -> RawWaker { noop_raw_waker() }
        
        // 虚函数表（vtable）：定义 Waker 的行为
        // 参数顺序：clone, wake, wake_by_ref, drop
        let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
        
        // 创建 RawWaker：data 是空指针（因为没有状态需要存储），vtable 定义行为
        RawWaker::new(std::ptr::null(), vtable)
    }

    // ========== 第三步：从 RawWaker 创建安全的 Waker ==========
    // 
    // SAFETY 安全性说明：
    // - noop_raw_waker() 返回一个有效的 RawWaker
    // - vtable 中的函数都是有效的（虽然什么都不做）
    // - RawWaker 的生命周期由我们管理，不会悬垂
    let waker = unsafe { Waker::from_raw(noop_raw_waker()) };
    
    // 创建 Context：包含 Waker，传递给 Future 的 poll 方法
    let mut cx = Context::from_waker(&waker);

    // ========== 第四步：忙循环轮询 Future ==========
    // 这是"轮询驱动"的核心：不断调用 poll，直到 Future 完成
    // 
    // 真正的执行器优化：
    // - 当 Future 返回 Pending 时，挂起线程（park）
    // - 当 Waker 被调用时，重新唤醒线程（unpark）
    // - 而不是浪费 CPU 空转
    loop {
        match future.as_mut().poll(&mut cx) {
            // Future 完成！返回结果
            Poll::Ready(value) => return value,
            
            // Future 未完成，稍后再试
            Poll::Pending => {
                // 真正的执行器会在这里：
                // - 挂起线程，等待 Waker.wake() 被调用
                // - 释放 CPU 给其他任务
                // 
                // 我们这里只是简单的让出 CPU 时间片
                // 这仍然会占用 CPU，但至少给其他线程机会运行
                std::thread::yield_now();
            }
        }
    }
}
fn main() {
    // let rt = Runtime::new().unwrap();
    println!("Hello, world!");
     let msg = block_on(CountdownFuture::new(5000*100000));
    println!("{}", msg);

    // let rt2 = Runtime::new().unwrap();
    // println!("Hello, world!");

    // let mut count = 5u32;
    // let msg2 = rt.block_on(poll_fn(|cx| {
    //     if count == 0 {
    //         Poll::Ready(5)
    //     } else {
    //         println!("{}...", count);
    //         count -= 1;
    //         cx.waker().wake_by_ref();
    //         Poll::Pending
    //     }
    // }));
    // println!("{}", msg2);
    //
    // // yield_now 示例：展示两个任务如何交替运行
    // println!("\n--- yield_now 示例 ---");
    // rt.block_on(async {
    //     // 任务 A：每 2 次让出
    //     let task_a = tokio::spawn(async {
    //         for i in 1..=6 {
    //             println!("任务 A: {}", i);
    //             if i % 2 == 0 {
    //                 tokio::task::yield_now().await;
    //             }
    //         }
    //         "A 完成"
    //     });
    //
    //     // 任务 B：每 2 次让出
    //     let task_b = tokio::spawn(async {
    //         for i in 1..=6 {
    //             println!("任务 B: {}", i);
    //             if i % 2 == 0 {
    //                 tokio::task::yield_now().await;
    //             }
    //         }
    //         "B 完成"
    //     });
    //
    //     let (a, b) = tokio::join!(task_a, task_b);
    //     println!("{}, {}", a.unwrap(), b.unwrap());
    // });

    // cpu_heavy_work 示例
    // println!("\n--- cpu_heavy_work 示例 ---");
    // let items: Vec<Item> = (0..250).map(|i| Item { value: i }).collect();
    // rt.block_on(async {
    //     let task_b = tokio::spawn(async {
    //         for i in 1..=300 {
    //             println!("任务 B: {}", i);
    //             if i % 2 == 0 {
    //                 tokio::task::yield_now().await;
    //             }
    //         }
    //         "B 完成"
    //     });
    //     let (a, b) = tokio::join!(cpu_heavy_work(&items), task_b);
    //     println!("{}", b.unwrap());
    //     println!("CPU 密集工作完成！");
    // });
}

// yield_now: voluntarily yield control to the executor
// Useful in CPU-heavy async loops to avoid starving other tasks

struct Item {
    value: i32,
}

fn process(item: &Item) {
    // 模拟 CPU 密集计算
    let _ = item.value * 2;
}

async fn cpu_heavy_work(items: &[Item]) {
    for (i, item) in items.iter().enumerate() {
        println!("A在工作{}", i);
        process(item); // CPU work

        // Every 100 items, yield to let other tasks run
        if i % 100 == 0 {
            tokio::task::yield_now().await;
        }
    }
}