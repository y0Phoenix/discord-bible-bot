use std::{sync::Arc, thread};
use chrono::{DateTime, Datelike, Duration, Local, Timelike, Weekday};

use dotenv::dotenv;
use poise::serenity_prelude as serenity;
use ::serenity::all::{ChannelId, CreateScheduledEvent, EventHandler, GetMessages, GuildId, Ready, ScheduledEvent, ScheduledEventType, Timestamp};
use tracing::info;

// use sqlx::{PgPool, Pool, Postgres};

pub const EVENT_NAME: &'static str = "Bible study and Prayer";
pub const EVENT_HOUR: u32 = 20;

pub const GUILD_ID: GuildId = GuildId::new(1422398523160264907);
pub const VC_CHANNEL_ID: ChannelId = ChannelId::new(1422398524037009541);
pub const SESSIONS_CHANNEL_ID: ChannelId = ChannelId::new(1424558096910647306);
pub const ADMIN_CHANNEL_ID: ChannelId = ChannelId::new(1424524338677420243);
struct Data {
    //db: Pool<Postgres>,
} // User data, which is stored and accessible in all command invocations
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

struct Handler;

#[serenity::async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: poise::serenity_prelude::Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        tokio::spawn(async move {
            let http = Arc::clone(&ctx.http);
            let mut eight_hour_msg_sent = false;
            let mut one_hour_msg_sent = false;

            loop {
                let http_clone = Arc::clone(&http);
                // Get the current local time
                let now = Local::now();
                // Find the next Monday
                let next_monday = {
                    let mut days_ahead = (Weekday::Mon.num_days_from_monday() as i64
                        - now.weekday().num_days_from_monday() as i64)
                        % 7;
                    if days_ahead <= 0 {
                        days_ahead += 7; // move to next week if today is Monday or later
                    }
                    now + Duration::days(days_ahead)
                };
                
                let scheduled_events = GUILD_ID.scheduled_events(http_clone.clone(), false).await.expect("Failed to get events");
                let bible_events: Vec<&ScheduledEvent> = scheduled_events.iter().filter(|event| event.name.clone() == EVENT_NAME.to_string()).collect();

                if now.weekday() == Weekday::Mon && (EVENT_HOUR - now.hour() == 8) && !bible_events.is_empty() && !eight_hour_msg_sent {
                    let _msg = SESSIONS_CHANNEL_ID.say(http_clone.clone(), "Hey @everyone, schedules session today starts in 8 hours").await.expect("Failed to send reminder message");
                    one_hour_msg_sent = true;
                }
                
                if now.weekday() == Weekday::Mon && (EVENT_HOUR - now.hour() == 1) && !bible_events.is_empty() && !one_hour_msg_sent {
                    let _msg = SESSIONS_CHANNEL_ID.say(http_clone.clone(), "Hey @everyone, schedules session today starts in 1 hour").await.expect("Failed to send reminder message");
                    one_hour_msg_sent = true;
                }

                if now.weekday() == Weekday::Tue && !bible_events.is_empty() {
                    for event in bible_events.iter() {
                        if event.start_time.day() < next_monday.day() as u8 {
                            if let Err(e) = GUILD_ID.delete_scheduled_event(http_clone.clone(), event.id).await {
                                let _ = ADMIN_CHANNEL_ID.say(http_clone.clone(),format!("@Eugene there was a problem deleting the old event on the server. Please do it manually I guess :nerd: {}", e)).await;
                            }
                        }
                    }
                }

                if bible_events.is_empty() {
                    info!("Creating event: {}", EVENT_NAME);
                    eight_hour_msg_sent = false;
                    one_hour_msg_sent = false;
    
                    // Build 8:00 AM Monday start and 9:00 AM end (1 hour duration)
                    let start_time_local: DateTime<Local> =
                        next_monday.date_naive().and_hms_opt(EVENT_HOUR, 0, 0).unwrap().and_local_timezone(Local).unwrap();
                    let end_time_local = start_time_local + Duration::hours(1);
    
                    // Convert to Discord-compatible timestamps
                    let start_time = Timestamp::from_unix_timestamp(start_time_local.timestamp()).unwrap();
                    let end_time = Timestamp::from_unix_timestamp(end_time_local.timestamp()).unwrap();
            
                    let builder = CreateScheduledEvent::new(
                            ScheduledEventType::Voice,
                            EVENT_NAME,
                            start_time,
                    )
                        .end_time(end_time)
                        .channel_id(VC_CHANNEL_ID)
                        .description("Weekly group discussion and prayer.");
            
                    let event = match GUILD_ID.create_scheduled_event(http_clone.clone(), builder).await {
                        Ok(event) => {
                            info!("✅ Created event: {} on {}", event.name, event.start_time.date());
                            event
                        },
                        Err(why) => {
                            ADMIN_CHANNEL_ID.say(http_clone.clone(), "Hey @Eugene. The bot is having an issue please fix").await.expect("Failed to send error message to aaron");
                            panic!("❌ Failed to create event: {:?}", why)
                        },
                    };

                    let event_url = format!("https://discord.com/events/{}/{}", event.guild_id, event.id);
                    
                    let channel_msgs = SESSIONS_CHANNEL_ID.messages(http_clone.clone(), GetMessages::new()).await.expect("Failed to retrieve messages from sessions channel");

                    for msg in channel_msgs.iter() {
                        let _ = msg.delete(http_clone.clone()).await;
                    }

                    SESSIONS_CHANNEL_ID.say(http_clone.clone(), format!("Hey @everyone, a new session has been scheduled. If you can join great, just click interested. \n{}", event_url).to_string()).await.unwrap();
                }

                thread::sleep(std::time::Duration::from_millis(60000));
                // thread::sleep(std::time::Duration::from_millis(5000));
            }
        });

    }
}

/// Displays your or another user's account creation date
#[poise::command(slash_command, prefix_command)]
async fn age(
    ctx: Context<'_>,
    #[description = "Selected user"] user: Option<serenity::User>,
) -> Result<(), Error> {
    let u = user.as_ref().unwrap_or_else(|| ctx.author());
    let response = format!("{}'s account was created at {}", u.name, u.created_at());
    ctx.say(response).await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt::init();
    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    //let db_url = std::env::var("DB_URL").expect("missing DB_URL");
    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    // connect to the db
    //let pool = PgPool::connect(&db_url)
    //    .await
    //    .expect("Failed to connect to db");
    //sqlx::migrate!("./migrations")
    //    .run(&pool)
    //    .await
    //    .expect("Couldn't run db migrations");

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![age()],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some(String::from("-")),
                ignore_bots: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data { 
                    //db: pool
                })
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .event_handler(Handler)
        .await;
    client.unwrap().start().await.unwrap();
}
