use async_std::task;
use trace::trace;

trace::init_depth_var!();

#[trace]
async fn squared(x: i32) -> i32 {
    x * x
}

#[async_trait::async_trait]
trait Log {
    async fn log(&self, message: &str) -> String;
}

#[async_trait::async_trait]
trait Cubed {
    async fn cubed(x: i32) -> i32;
}

struct Logger {
    level: String,
}

struct Math {}

#[trace]
#[async_trait::async_trait]
impl Log for Logger {
    async fn log(&self, message: &str) -> String {
        format!("[{}] {message}", self.level)
    }
}

#[trace]
#[async_trait::async_trait]
impl Cubed for Math {
    async fn cubed(x: i32) -> i32 {
        squared(squared(x).await).await
    }
}

fn main() {
    task::block_on(async {
        squared(64).await;
        let logger = Logger {
            level: "DEBUG".to_string(),
        };
        logger.log("something happened").await;
        Math::cubed(32).await;
    });
}

#[cfg(test)]
#[macro_use]
mod trace_test;

#[cfg(test)]
trace_test!(test_async, main());
