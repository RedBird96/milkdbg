use flume::Sender;

mod js;

pub enum Commands {
    Resolve(u32, serde_json::Value), //resolver, value
    Run(String, Sender<()>),
}

#[derive(Debug)]
pub enum Events {
    NativeCode(u32, String, Vec<serde_json::Value>), // resolver, function, arguments
}

pub struct Script {
    pub sender: Sender<Commands>,
}

impl Script {
    pub async fn send_async(&mut self, cmd: Commands) {
        let _ = self.sender.send_async(cmd).await;
    }
}

pub fn start(sender: flume::Sender<Events>) -> Script {
    let sender = js::JavascriptEngine::spawn(sender);
    Script { sender }
}
