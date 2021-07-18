use bytes::Bytes;
use mini_redis::client;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

// 複数の異なるコマンドは1つのチャネルを通して「多重化 (multiplexed)」される
#[derive(Debug)]
enum Command {
    Get {
        key: String,
        resp: Responder<Option<Bytes>>,
    },
    Set {
        key: String,
        val: Bytes,
        resp: Responder<()>,
    },
}

/// リクエストを送る側が生成する。
/// "マネージャー" タスクがレスポンスをリクエスト側に送り返すために使われる
type Responder<T> = oneshot::Sender<mini_redis::Result<T>>;

#[tokio::main]
async fn main() {
    // 最大 32 のキャパシティをもったチャネルを作成
    let (tx, mut rx) = mpsc::channel(32);

    // `rx` の所有権をタスクへとムーブするために `move` キーワードを付ける
    let manager = tokio::spawn(async move {
        // サーバーへのコネクションを確立する
        let mut client = client::connect("127.0.0.1:6379").await.unwrap();

        // メッセージの受信を開始
        while let Some(cmd) = rx.recv().await {
            use Command::*;

            match cmd {
                Get { key, resp } => {
                    let res = client.get(&key).await;
                    let _ = resp.send(res);
                }
                Set { key, val, resp } => {
                    let res = client.set(&key, val).await;
                    let _ = resp.send(res);
                }
            }
        }
    });

    let tx2 = tx.clone();

// タスク1は "get" を、タスク2は "set" を担当する
    let t1 = tokio::spawn(async move {
        let (resp_tx, resp_rx) = oneshot::channel();

        let cmd = Command::Get {
            key: "hello".to_string(),
            resp: resp_tx,
        };

        tx.send(cmd).await.unwrap();

        let res = resp_rx.await;
        println!("GOT = {:?}", res);
    });

    let t2 = tokio::spawn(async move {
        let (resp_tx, resp_rx) = oneshot::channel();

        let cmd = Command::Set {
            key: "foo".to_string(),
            val: "bar".into(),
            resp: resp_tx,
        };

        // SET リクエストを送信
        tx2.send(cmd).await.unwrap();

        // レスポンスが来るのを待つ
        let res = resp_rx.await;
        println!("GOT = {:?}", res);
    });

    t1.await.unwrap();
    t2.await.unwrap();
    manager.await.unwrap();
}
