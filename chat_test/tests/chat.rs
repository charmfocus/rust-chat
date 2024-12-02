use anyhow::Result;
use chat_core::{Chat, ChatType, Message};
use futures::StreamExt;
use reqwest::{
    multipart::{Form, Part},
    StatusCode,
};
use reqwest_eventsource::{Event, EventSource};
use serde::Deserialize;
use std::{net::SocketAddr, time::Duration};
use tokio::{net::TcpListener, time::sleep};

const WILD_ADDR: &str = "0.0.0.0:0";

#[derive(Debug, Deserialize)]
struct AuthToken {
    token: String,
}

struct ChatServer {
    addr: SocketAddr,
    token: String,
    client: reqwest::Client,
}

struct NotifyServer;

#[tokio::test]
async fn chat_ser_should_work() -> Result<()> {
    let (tdb, state) = chat_server::AppState::new_for_test().await?;
    let chat_server = ChatServer::new(state).await?;
    let db_url = tdb.url();
    NotifyServer::new(&db_url, &chat_server.token).await?;
    let chat = chat_server.create_chat().await?;
    let _msg = chat_server.create_message(chat.id as u64).await?;
    sleep(Duration::from_secs(1)).await;
    Ok(())
}

impl ChatServer {
    async fn new(state: chat_server::AppState) -> Result<Self> {
        let app = chat_server::get_router(state).await?;
        let listener = TcpListener::bind(WILD_ADDR).await?;
        let addr = listener.local_addr()?;

        tokio::spawn(async move {
            axum::serve(listener, app.into_make_service())
                .await
                .unwrap();
        });

        let client = reqwest::Client::new();

        let mut ret = Self {
            addr,
            client,
            token: "".to_string(),
        };

        ret.token = ret.signin().await?;

        Ok(ret)
    }

    async fn signin(&self) -> Result<String> {
        let res = self
            .client
            .post(format!("http://{}/api/signin", self.addr))
            .header("Content-Type", "application/json")
            .body(r#"{"email": "charmfocus@gmail.com", "password":"123456"}"#)
            .send()
            .await?;

        assert_eq!(res.status(), 200);
        let ret = res.json::<AuthToken>().await?;
        Ok(ret.token)
    }

    async fn create_chat(&self) -> Result<Chat> {
        let res = self
            .client
            .post(format!("http://{}/api/chats", self.addr))
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.token))
            .body(r#"{"name": "test", "members": [1,2], "public": false}"#);

        let res = res.send().await?;
        assert_eq!(res.status(), StatusCode::CREATED);

        let chat = res.json::<Chat>().await?;
        assert_eq!(chat.name.as_ref().unwrap(), "test");
        assert_eq!(chat.members, vec![1, 2]);
        assert_eq!(chat.r#type, ChatType::PrivateChannel);
        Ok(chat)
    }

    async fn create_message(&self, chat_id: u64) -> Result<Message> {
        // upload file
        let data = include_bytes!("../Cargo.toml");
        let files = Part::bytes(data)
            .file_name("Cargo.toml")
            .mime_str("text/plain")?;
        let form = Form::new().part("file", files);
        let res = self
            .client
            .post(format!("http://{}/api/upload", self.addr))
            .header("Authorization", format!("Bearer {}", self.token))
            .multipart(form)
            .send()
            .await?;

        assert_eq!(res.status(), StatusCode::OK);
        let ret: Vec<String> = res.json().await?;

        let body = serde_json::to_string(&serde_json::json!({
            "content": "hello",
            "files": ret
        }))?;

        let res = self
            .client
            .post(format!("http://{}/api/chats/{}", self.addr, chat_id))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await?;
        assert_eq!(res.status(), StatusCode::CREATED);
        let message = res.json::<Message>().await?;
        assert_eq!(message.content, "hello");
        assert_eq!(message.files, ret);
        assert_eq!(message.sender_id, 1);
        assert_eq!(message.chat_id, chat_id as i64);

        Ok(message)
    }
}

impl NotifyServer {
    async fn new(db_url: &str, token: &str) -> Result<Self> {
        let mut config = notify_server::AppConfig::load()?;
        config.server.db_url = db_url.to_string();
        let app = notify_server::get_router(config).await?;
        let listener = TcpListener::bind(WILD_ADDR).await?;
        let addr = listener.local_addr()?;

        tokio::spawn(async move {
            axum::serve(listener, app.into_make_service())
                .await
                .unwrap();
        });

        let mut es = EventSource::get(format!("http://{}/events?token={}", addr, token));

        tokio::spawn(async move {
            while let Some(event) = es.next().await {
                match event {
                    Ok(Event::Open) => println!("Connection Open!"),
                    Ok(Event::Message(message)) => match message.event.as_str() {
                        "NewChat" => {
                            let chat: Chat = serde_json::from_str(&message.data).unwrap();
                            assert_eq!(chat.name.as_ref().unwrap(), "test");
                            assert_eq!(chat.members, vec![1, 2]);
                            assert_eq!(chat.r#type, ChatType::PrivateChannel);
                        }
                        "NewMessage" => {
                            let message: Message = serde_json::from_str(&message.data).unwrap();
                            assert_eq!(message.content, "hello");
                            assert_eq!(message.files.len(), 1);
                            assert_eq!(message.sender_id, 1);
                        }
                        _ => {
                            panic!("unexpected event: {:?}", message);
                        }
                    },
                    Err(err) => {
                        println!("Error: {:?}", err);
                        es.close();
                    }
                }
            }
        });

        Ok(Self)
    }
}
