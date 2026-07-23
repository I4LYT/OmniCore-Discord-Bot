# OmniCore Discord Bot

OmniCore Discord Bot is a multi-purpose Discord bot written in Rust. It has Ollama integration, letting your Discord server feel alive.

## Building
just do `cargo build --release` inside of the project folder, the binary will be in `target/release/omnicore_bot`. You do need Rust though, and if you're on windows, it will produce it with a .exe at the end. I discourage windows so use Docker or WSL
## Running
### Binary Directly
Build the binary first then simply run it, you may need to make a .env file with all the required environment variables.
### Docker
Run this in Docker Compose:
```yaml
services:
  omnicore_bot:
    build: .
    container_name: omnicore_bot
    image: ghcr.io/Shreshtgaming606/omnicore_bot:latest
    environment:
      - DISCORD_TOKEN={your token here}
      - OLLAMA_BASE_URL={ollama url here}
      - MONGODB_URI={mongodb uri here}
      - OLLAMA_MODEL={ollama model here}
      - BOT_OWNERS={ids here}
    restart: always
```