
use std::future::{poll_fn, Future};
use std::pin::Pin;
use std::task::{Context, Poll};
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

fn main() {
    let rt = Runtime::new().unwrap();
    println!("Hello, world!");
     let msg = rt.block_on(CountdownFuture::new(5));
    println!("{}", msg);

    // let rt2 = Runtime::new().unwrap();
    // println!("Hello, world!");

    let mut count = 5u32;
    let msg2 = rt.block_on(poll_fn(|cx| {
        if count == 0 {
            Poll::Ready(5)
        } else {
            println!("{}...", count);
            count -= 1;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }));
    println!("{}", msg2);

    // yield_now 示例：展示两个任务如何交替运行
    println!("\n--- yield_now 示例 ---");
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
    println!("\n--- cpu_heavy_work 示例 ---");
    let items: Vec<Item> = (0..250).map(|i| Item { value: i }).collect();
    rt.block_on(async {
        let task_b = tokio::spawn(async {
            for i in 1..=300 {
                println!("任务 B: {}", i);
                if i % 2 == 0 {
                    tokio::task::yield_now().await;
                }
            }
            "B 完成"
        });
        let (a, b) = tokio::join!(cpu_heavy_work(&items), task_b);
        println!("{}", b.unwrap());
        println!("CPU 密集工作完成！");
    });
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