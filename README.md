# Discord Bot with Clustering Support

A feature-rich Discord bot with ticket system, moderation tools, announcements, and clustering capability for high availability.

## Features

- **Clustering**: Multiple bot instances with automatic leader election
- **Ticket System**: Create and manage support tickets
- **Moderation**: Kick, ban, and manage members
- **Announcements**: Send server-wide announcements
- **Logging**: Comprehensive logging for various events
- **Configuration**: Flexible per-guild configuration

## Setup

1. Clone this repository
2. Install Rust (latest stable version)
3. Create `config.toml` and `cluster.toml` files (see Configuration section)
4. Set `DISCORD_TOKEN` environment variable
5. Run with `cargo run`

## Configuration

### Cluster Configuration (`cluster.toml`)
```toml
[cluster]
instance_id = "unique-instance-name"  # Unique identifier for this instance
priority = 1                         # Leadership priority (higher = more likely to be leader)
```

### Bot Confuration
The bot automatically creates this file with default values if it doesn't exist.
Example structure:
```toml
[guild_id]
logging_channel = 1234567890         # Default logging channel
member_log_channel = 1234567890      # Member join/leave logs
ticket_log_channel = 1234567890      # Ticket activity logs
mod_log_channel = 1234567890         # Moderation action logs
announcement_channel = 1234567890    # Announcement channel
boot_quit_channel = 1234567890       # Bot startup/shutdown notifications
ticket_category = 1234567890         # Category for ticket channels
ticket_roles = [1234567890]          # Roles with ticket access
ticket_exempt_role = 1234567890      # Role exempt from seeing ticket message
```

## Commands

### General Commands
* `help [command]` - Shows help menu (leader-only)
* `vote <choice>` - Vote for something
* `getvotes [choice]` - Show vote counts

### Ticket Commands
* `ticket [issue]` - Create a new support ticket
* `closeticket [reason]` - Close the current ticket (admin-only)

### Moderation Commands
* `kick <user> [reason]` - Kick a user (admin-only)
* `ban <user> [reason] [delete_message_days]` - Ban a user (admin-only)
* `announce <message>` - Make an announcement (admin-only, leader-only)

### Configuration Commands (admin-only, leader-only)
* `config` - Show configuration commands
* `set_log_channel <type> <channel>` - Set logging channel
    * Types: boot, member, ticket, mod, announcement, or default
* `set_announcement_channel <channel>` - Set announcement channel
* `set_ticket_log_channel <channel>` - Set ticket logging channel
* `set_member_log_channel <channel>` - Set member join/leave logging channel
* `log_channel <channel>` - Set default logging channel
* `ticket_category <category>` - Set ticket category
* `add_ticket_role <role>` - Add role to ticket access
* `remove_ticket_role <role>` - Remove role from ticket access
* `ticket_message <text_file>` - Set ticket message template (upload .txt file)
* `ticket_exempt_role <role>` - Set role exempt from ticket message
* `remove_ticket_exempt_role` - Remove ticket exempt role
* `list_ticket_roles` - List all ticket access roles

### Owner Commands
* `quit - Shutdown the bot (owner-only)
* `writeconfig - Force save config to disk (owner-only)

## Clustering
The bot supports multiple instances with automatic leader election:
* Instances communicate via a dedicated Discord channel
* Heartbeats are sent every 10 seconds
* Leader timeout is 60 seconds
* Highest priority instance becomes leader (with oldest instance as tiebreaker)
* Only the leader executes certain commands

## Event Logging
The bot logs these events to configured channels:
* Bot startup/shutdown
* Member joins/leaves
* Ticket creation/closing
* Moderation actions (kicks/bans)
* Announcements

## Ticket System Features
* Customizable ticket message templates
* Role-based access control
* Exempt roles that don't see the ticket message
* Automatic channel creation with proper permissions
* Comprehensive logging

## Requirements
* Rust 1.70+
* Tokio runtime
* Discord bot token with these intents:
    * Guilds
    * Guild Members
    * Guild Messages
    * Message Content

License
