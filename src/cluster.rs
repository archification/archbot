use crate::Error;
use poise::serenity_prelude::{self as serenity, ChannelId};
use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use crate::utils::update_config_from_str;

const HEARTBEAT_INTERVAL: u64 = 10;
const LEADER_TIMEOUT: u64 = 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceInfo {
    pub instance_id: String,
    pub priority: i32,
    pub last_seen: u64,
    pub is_leader: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClusterMessage {
    Heartbeat(InstanceInfo),
    ConfigUpdate(String),
    LeaderAnnouncement(String),
    TicketTemplateUpdate {
        guild_id: u64,
        content: String,
    },
    ConfigRequest,
}

pub struct ClusterState {
    pub instances: HashMap<String, InstanceInfo>,
    pub current_leader: Option<String>,
    pub my_instance_id: String,
    pub my_priority: i32,
    pub is_leader: bool,
    pub coordination_channel_id: u64,
}

impl ClusterState {
    pub fn new(instance_id: String, priority: i32, coordination_channel_id: u64) -> Self {
        ClusterState {
            instances: HashMap::new(),
            current_leader: None,
            my_instance_id: instance_id,
            my_priority: priority,
            is_leader: false,
            coordination_channel_id,
        }
    }

    pub fn update_instance(&mut self, info: InstanceInfo) {
        self.instances.insert(info.instance_id.clone(), info);
        self.check_leader();
    }

    pub fn check_leader(&mut self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.instances.retain(|_, info| now - info.last_seen <= LEADER_TIMEOUT);
        let best_candidate = self.instances.values()
            .filter(|info| now - info.last_seen <= LEADER_TIMEOUT)
            .max_by_key(|info| (
                std::cmp::Reverse(info.priority),
                info.last_seen
            ));
        let new_leader_id = best_candidate.map(|info| info.instance_id.clone());
        let leadership_changed = new_leader_id != self.current_leader;
        if leadership_changed {
            self.current_leader = new_leader_id.clone();
            self.is_leader = new_leader_id.as_deref() == Some(&self.my_instance_id);
        }
        leadership_changed
    }
}

pub async fn start_cluster_loop(
    ctx: serenity::Context,
    _data: Arc<Mutex<crate::Data>>,
    cluster_state: Arc<Mutex<ClusterState>>,
) {
    let coordination_channel_id = {
        let state = cluster_state.lock().await;
        state.coordination_channel_id
    };
    let cluster_channel = ChannelId::new(coordination_channel_id);
    if let Err(e) = cluster_channel.send_message(&ctx.http,
        serenity::CreateMessage::new()
            .content(serde_json::to_string(&ClusterMessage::ConfigRequest).unwrap())
    ).await {
        println!("Failed to send config request: {}", e);
    }
    sleep(Duration::from_secs(5)).await;
    loop {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let became_leader = {
            let mut state = cluster_state.lock().await;
            let leadership_changed = state.check_leader();
            leadership_changed && state.is_leader
        };
        let heartbeat = {
            let state = cluster_state.lock().await;
            ClusterMessage::Heartbeat(InstanceInfo {
                instance_id: state.my_instance_id.clone(),
                priority: state.my_priority,
                last_seen: now,
                is_leader: state.is_leader,
            })
        };
        if let Err(e) = cluster_channel.send_message(&ctx.http,
            serenity::CreateMessage::new()
                .content(serde_json::to_string(&heartbeat).unwrap())
        ).await {
            println!("Failed to send heartbeat: {}", e);
        }
        if became_leader {
            let announcement = ClusterMessage::LeaderAnnouncement(
                cluster_state.lock().await.my_instance_id.clone()
            );
            if let Err(e) = cluster_channel.send_message(
                &ctx.http,
                serenity::CreateMessage::new()
                    .content(serde_json::to_string(&announcement).unwrap())
            ).await {
                println!("Failed to send leader announcement: {}", e);
            }
        }
        sleep(Duration::from_secs(HEARTBEAT_INTERVAL)).await;
    }
}

pub async fn handle_cluster_message(
    ctx: &serenity::Context,
    message: &serenity::Message,
    cluster_state: Arc<Mutex<ClusterState>>,
    data: Arc<Mutex<crate::Data>>,
) -> Result<(), Error> {
    let coordination_channel_id = {
        let state = cluster_state.lock().await;
        state.coordination_channel_id
    };
    if message.channel_id != coordination_channel_id {
        return Ok(());
    }
    let cluster_msg: ClusterMessage = match serde_json::from_str(&message.content) {
        Ok(msg) => msg,
        Err(_) => return Ok(()),
    };
    match cluster_msg {
        ClusterMessage::ConfigRequest => {
            let state = cluster_state.lock().await;
            if state.is_leader {
                let config_str = crate::utils::get_config_as_string()?;
                let cluster_channel = ChannelId::new(coordination_channel_id);
                cluster_channel.send_message(
                    ctx,
                    serenity::CreateMessage::new()
                        .content(serde_json::to_string(
                            &ClusterMessage::ConfigUpdate(config_str)
                        )?)
                ).await?;
            }
        }
        ClusterMessage::Heartbeat(info) => {
            let mut state = cluster_state.lock().await;
            state.update_instance(info);
            state.check_leader();
        }
        ClusterMessage::ConfigUpdate(config) => {
            let state = cluster_state.lock().await;
            if state.is_leader {
                return Ok(());
            }
            let _data = data.lock().await;
            if let Err(e) = update_config_from_str(&config) {
                println!("Failed to update config: {}", e);
            }
        }
        ClusterMessage::LeaderAnnouncement(instance_id) => {
            let mut state = cluster_state.lock().await;
            if let Some(info) = state.instances.get_mut(&instance_id) {
                info.is_leader = true;
            }
            state.current_leader = Some(instance_id);
            state.is_leader = state.current_leader.as_deref() == Some(&state.my_instance_id);
        }
        ClusterMessage::TicketTemplateUpdate { guild_id, content } => {
            let path = crate::utils::get_ticket_template_path(guild_id);
            if let Err(e) = std::fs::create_dir_all("./ticket_templates")
                .and_then(|_| std::fs::write(path, content))
            {
                println!("Failed to save ticket template for guild {}: {}", guild_id, e);
            }
        }
    }
    Ok(())
}
