use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Clone, Serialize, Deserialize)]
pub struct BrokerEnvelope {
    pub instance_id: String,
    pub payload: String,
}

#[derive(Clone)]
pub enum Broker {
    Local,
    Redis { client: redis::Client },
}

impl Broker {
    pub async fn connect() -> Self {
        let url = std::env::var("REDIS_URL").ok();
        let Some(url) = url else {
            info!("REDIS_URL not set; single-instance mode");
            return Self::Local;
        };
        match redis::Client::open(url.as_str()) {
            Ok(client) => {
                if client.get_multiplexed_async_connection().await.is_ok() {
                    info!("connected to Redis pub/sub broker");
                    Self::Redis { client }
                } else {
                    warn!("Redis unreachable; falling back to local broadcast");
                    Self::Local
                }
            }
            Err(e) => {
                warn!("Redis client error: {e}; falling back to local broadcast");
                Self::Local
            }
        }
    }

    pub async fn publish(&self, room_id: &str, envelope: &BrokerEnvelope) {
        let Broker::Redis { client } = self else {
            return;
        };
        let channel = format!("excaildraw:room:{room_id}");
        if let Ok(json) = serde_json::to_string(envelope) {
            if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
                use redis::AsyncCommands;
                let _: Result<(), _> = conn.publish(channel, json).await;
            }
        }
    }

    pub fn spawn_subscriber(
        broker: Broker,
        forward: Arc<dyn Fn(String, BrokerEnvelope) + Send + Sync>,
    ) {
        let Broker::Redis { client } = broker else {
            return;
        };
        tokio::spawn(async move {
            if let Ok(mut pubsub) = client.get_async_pubsub().await {
                if pubsub.psubscribe("excaildraw:room:*").await.is_ok() {
                    use futures::StreamExt;
                    let mut stream = pubsub.on_message();
                    while let Some(msg) = stream.next().await {
                        if let Ok(payload) = msg.get_payload::<String>() {
                            if let Ok(envelope) = serde_json::from_str::<BrokerEnvelope>(&payload) {
                                if let Ok(channel) = msg.get_channel::<String>() {
                                    if let Some(room) =
                                        channel.strip_prefix("excaildraw:room:")
                                    {
                                        forward(room.to_string(), envelope);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
    }
}
