use crate::{Context, Error};
use std::fs;
use std::path::Path;
use toml::Value;
use poise::serenity_prelude as serenity;

async fn autocomplete_stats<'a>(
    ctx: Context<'a>,
    partial: &'a str,
) -> impl Iterator<Item = String> + 'a {
    let guild_id = ctx.guild_id().unwrap_or_default();
    let stats = crate::utils::get_custom_stats(guild_id.into()).await;
    stats.into_iter().filter(move |s| s.starts_with(&partial.to_lowercase()))
}

pub async fn modify_user_stat(guild_id: u64, user_id: u64, stat_name: &str, amount: i64) -> Result<i64, Error> {
    std::fs::create_dir_all("./user_stats")?;
    let filename = format!("./user_stats/stats_{}_{}.toml", guild_id, user_id);
    let path = Path::new(&filename);
    let mut doc = if path.exists() {
        fs::read_to_string(path)?.parse::<Value>().unwrap_or(Value::Table(toml::value::Table::new()))
    } else {
        Value::Table(toml::value::Table::new())
    };
    let table = doc.as_table_mut().unwrap();
    let current = table.get(stat_name).and_then(|v| v.as_integer()).unwrap_or(0);
    let new_val = current + amount;
    table.insert(stat_name.to_string(), Value::Integer(new_val));
    fs::write(path, toml::to_string_pretty(&doc)?)?;
    Ok(new_val)
}

pub async fn get_user_stat(guild_id: u64, user_id: u64, stat_name: &str) -> i64 {
    let filename = format!("./user_stats/stats_{}_{}.toml", guild_id, user_id);
    let path = Path::new(&filename);
    if path.exists() {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(doc) = content.parse::<Value>() {
                if let Some(val) = doc.get(stat_name).and_then(|v| v.as_integer()) {
                    return val;
                }
            }
        }
    }
    0
}

#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn stat(
    ctx: Context<'_>,
    #[description = "The stat to record"]
    #[autocomplete = "autocomplete_stats"]
    stat_name: String,
    #[description = "The amount to add (default 1)"]
    amount: Option<i64>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let user_id = ctx.author().id;
    let stat_lower = stat_name.to_lowercase();
    let allowed_stats = crate::utils::get_custom_stats(guild_id.into()).await;
    if !allowed_stats.contains(&stat_lower) {
        ctx.say(format!("❌ `{}` is not a tracked stat on this server.", stat_name)).await?;
        return Ok(());
    }
    let add_amount = amount.unwrap_or(1);
    let new_total = modify_user_stat(guild_id.into(), user_id.into(), &stat_lower, add_amount).await?;
    ctx.say(format!("📈 Added {} to your `{}` stat! New total: **{}**", add_amount, stat_lower, new_total)).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn viewstat(
    ctx: Context<'_>,
    #[description = "The stat to view"]
    #[autocomplete = "autocomplete_stats"]
    stat_name: String,
    #[description = "User to view (defaults to you)"]
    user: Option<serenity::User>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let target_user = user.as_ref().unwrap_or(ctx.author());
    let stat_lower = stat_name.to_lowercase();
    let total = get_user_stat(guild_id.into(), target_user.id.into(), &stat_lower).await;
    ctx.say(format!("📊 {} has **{}** `{}`", target_user.name, total, stat_lower)).await?;
    Ok(())
}
