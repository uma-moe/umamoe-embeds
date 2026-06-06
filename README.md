# umamoe-embeds

`umamoe-embeds` is a small Rust edge component for rich link previews on an Angular SPA.

It lets the same public URL behave correctly for both humans and embed crawlers:

```text
GET /circles/772781438
User-Agent: Discordbot/2.0
-> small HTML document with Open Graph / Twitter Card metadata

GET /circles/772781438
User-Agent: normal browser
-> proxied Angular SPA shell
```

The shared URL stays canonical. Only `og:image` points at an internal generated PNG route:

```text
https://uma.moe/__embeds/images/circle/772781438.png
```

## Supported Routes

Route-specific metadata is generated for:

- `/profile/:accountId`
- `/profile/:accountId/veterans`
- `/profile/:accountId/cm`
- `/profile/:accountId/achievements`
- `/profile/:accountId/titles`
- `/circles/:id`
- `/database`
- `/timeline`
- `/tierlist`
- `/rankings`
- `/activity`
- `/tools`
- `/tools/statistics`
- `/tools/lineage-planner`
- `/privacy-policy`
- `/`

Unknown Angular routes get a generic `uma.moe` card. API, asset, resource, ingest, and internal embed paths are never intercepted.

## Configuration

Environment variables:

```text
UMAMOE_EMBEDS_BIND=0.0.0.0:8080
UMAMOE_PUBLIC_BASE_URL=https://uma.moe
UMAMOE_FRONTEND_ORIGIN=http://umamoe-frontend-shell:80
UMAMOE_API_BASE_URL=http://umamoe-api:8080
UMAMOE_EMBEDS_DEBUG_QUERY_KEY=__embed
UMAMOE_EMBEDS_BOT_USER_AGENT_TOKENS=Discordbot,Twitterbot,Slackbot,facebookexternalhit,Facebot,LinkedInBot,WhatsApp,TelegramBot
```

`UMAMOE_FRONTEND_ORIGIN` must point at the static Angular shell origin. `UMAMOE_API_BASE_URL` should point at the backend API origin, preferably an internal address. Avoid setting either one to the public `https://uma.moe` hostname if Cloudflare routes that hostname back to the embed service, or the service can loop into itself.

## Testing

Force embed HTML without spoofing a bot:

```text
https://uma.moe/circles/772781438?__embed=1
```

Bot-style curl:

```bash
curl -A "Discordbot/2.0" https://uma.moe/circles/772781438
```

Generated image:

```bash
curl https://uma.moe/__embeds/images/circle/772781438.png --output circle.png
```

## Caching

The bot HTML response sets:

```http
Cache-Control: no-store
Vary: User-Agent
```

Do not let Cloudflare cache route HTML by path only, because `/circles/772781438` returns different HTML for embed crawlers and browsers. The generated PNG image route is safe to cache briefly.

Recommended Cloudflare shape:

- Bypass cache for normal SPA HTML routes.
- Bypass cache for bot HTML responses.
- Cache `/__embeds/images/*` for a short time.
- Keep `/assets/*` immutable/static as before.

## Local Run

From this folder:

```bash
cargo run
```

With a local Angular dev server:

```bash
set UMAMOE_FRONTEND_ORIGIN=http://127.0.0.1:4200
set UMAMOE_API_BASE_URL=https://uma.moe
cargo run
```
