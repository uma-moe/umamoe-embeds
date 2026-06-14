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
- `/database?filters=...`
- `/database?trainer_id=...`
- `/timeline`
- `/tierlist`
- `/rankings`
- `/activity`
- `/tools`
- `/tools/statistics`
- `/tools/lineage-planner`
- `/tools/lineage-planner?tree=...`
- `/privacy-policy`
- `/`

Unknown Angular routes get a generic `uma.moe` card. API, asset, resource, ingest, and internal embed paths are never intercepted.

Database URLs with shared filter state are resolved through the search API. The embed service decodes the Angular `filters=` state, translates it into `/search/query?page=0&limit=1&search_type=inheritance`, and renders the top inheritance result in the preview. Direct copied params such as `trainer_id`, `trainer_name`, `query`, `blue_sparks`, `main_parent_id`, and other backend search params are also forwarded. When `umamoe-resources` is available, the embed service pulls factors, skills, affinity data, and race mappings so compact race scheduler filters can affect race affinity in the bot preview.

Lineage planner URLs with shared `tree=` state preserve the encoded URL in canonical Open Graph metadata and render a decoded planner card with the shared characters, sparks, race wins, and affinity data when resources are available.

## Configuration

Environment variables:

```text
UMAMOE_EMBEDS_BIND=0.0.0.0:8080
UMAMOE_PUBLIC_BASE_URL=https://uma.moe
UMAMOE_FRONTEND_ORIGIN=http://umamoe-frontend-shell:80
UMAMOE_ASSET_BASE_URL=https://uma.moe/assets
UMAMOE_API_BASE_URL=http://umamoe-api:8080
UMAMOE_SEARCH_BASE_URL=http://umamoe-search:3202
UMAMOE_RESOURCES_BASE_URL=http://umamoe-resources:3204/resources
UMAMOE_RESOURCES_API_TOKEN=
UMAMOE_EMBEDS_DEBUG_QUERY_KEY=__embed
UMAMOE_EMBEDS_BOT_USER_AGENT_TOKENS=Discordbot,Twitterbot,Slackbot,facebookexternalhit,Facebot,LinkedInBot,WhatsApp,TelegramBot
UMAMOE_EMBEDS_IMAGE_CACHE_MAX_AGE_SECONDS=300
UMAMOE_EMBEDS_IMAGE_CACHE_STALE_SECONDS=86400
UMAMOE_EMBEDS_IMAGE_CACHE_MAX_ENTRIES=256
UMAMOE_EMBEDS_RENDER_MAX_CONCURRENCY=1
UMAMOE_EMBEDS_CHROMIUM_DEBUG_PORT=
UMAMOE_EMBEDS_CHROMIUM_STARTUP_TIMEOUT_SECONDS=45
UMAMOE_EMBEDS_CHROMIUM_RENDER_TIMEOUT_SECONDS=15
```

`UMAMOE_FRONTEND_ORIGIN` must point at the static Angular shell origin. `UMAMOE_ASSET_BASE_URL` points at the public static assets directory and defaults to `https://uma.moe/assets`, so local Docker previews can reuse production character, support-card, rank, and logo assets. `UMAMOE_API_BASE_URL` defaults to the internal Docker backend origin `http://umamoe-backend:3001`. `UMAMOE_SEARCH_BASE_URL` defaults to the internal Docker search origin `http://umamoe-search:3202`. `UMAMOE_RESOURCES_BASE_URL` must point at the internal Docker resources service `/resources` origin and defaults to `http://umamoe-resources:3204/resources`; do not point it at the public `/resources` route because that route is browser-proof protected. `UMAMOE_RESOURCES_API_TOKEN` is optional and is only needed when using a protected resources endpoint. Avoid setting API/search/resources to the public `https://uma.moe` hostname if Cloudflare routes that hostname back to the embed service, or the service can loop into itself.

The image renderer keeps a Chromium process warm and opens short-lived DevTools tabs for screenshots. `UMAMOE_EMBEDS_RENDER_MAX_CONCURRENCY` defaults to `1` so bot bursts do not fan out into many simultaneous Chromium renders. PNGs are cached in-process for `UMAMOE_EMBEDS_IMAGE_CACHE_MAX_AGE_SECONDS` and can be served stale for `UMAMOE_EMBEDS_IMAGE_CACHE_STALE_SECONDS` while a refresh runs. If Chromium is busy and there is no stale entry, the service falls back to the cheaper Rust PNG renderer instead of queueing unbounded work. The Docker image installs Chromium; the host only needs Chromium installed if you run the Rust binary directly outside Docker. On slow/headless servers, increase `UMAMOE_EMBEDS_CHROMIUM_STARTUP_TIMEOUT_SECONDS`.

For the local Docker stack, `local.env` is used by `../umamoe-backend/compose.local.yml`.

## Testing

Force embed HTML without spoofing a bot:

```text
https://uma.moe/circles/772781438?__embed=1
https://uma.moe/database?filters=eyJic3MiOjl9&__embed=1
https://uma.moe/database?trainer_id=540903147493&__embed=1
https://uma.moe/tools/lineage-planner?tree=...&__embed=1
```

Forced debug embed pages stay viewable in a normal browser. Bot-rendered embed HTML includes a JavaScript redirect to the canonical `uma.moe` page so humans who land on an intermediate preview are forwarded immediately.

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

Default cache behavior:

- Bot/debug HTML: `Cache-Control: no-store` and `Vary: User-Agent`.
- PNG image responses: `Cache-Control: public, max-age=300, stale-while-revalidate=86400`.
- In-process PNG cache: 5 minutes fresh, then up to 24 hours stale by default.

## Nginx Routing

Add the shared maps/zones in the nginx `http` context, outside any `server` block:

```nginx
proxy_cache_path /var/cache/nginx/umamoe-embed-images
    levels=1:2
    keys_zone=umamoe_embed_images:50m
    max_size=1g
    inactive=1d
    use_temp_path=off;

map $http_user_agent $umamoe_embed_bot {
    default 0;
    "~*(Discordbot|Twitterbot|Slackbot|facebookexternalhit|Facebot|LinkedInBot|WhatsApp|TelegramBot|SkypeUriPreview|Pinterestbot|redditbot|Tumblr|Viber|Line|Embedly|Iframely|vkShare|Mastodon|Misskey|Bluesky)" 1;
}

map $arg___embed $umamoe_embed_debug {
    default 0;
    "1" 1;
    "true" 1;
}

map "$umamoe_embed_bot$umamoe_embed_debug" $umamoe_use_embeds {
    default 0;
    "~1" 1;
}

map $http_cf_connecting_ip $umamoe_limit_ip {
    default $http_cf_connecting_ip;
    "" $binary_remote_addr;
}

limit_req_zone $umamoe_limit_ip zone=umamoe_embed_html:10m rate=10r/s;
limit_req_zone $umamoe_limit_ip zone=umamoe_embed_images:10m rate=5r/s;
```

For beta, add this before the normal `location /` fallback and keep `/api/`, `/search/`, `/resources/`, `/assets/`, and other service locations more specific than `/`:

```nginx
location ^~ /__embeds/images/ {
    limit_req zone=umamoe_embed_images burst=20 nodelay;

    proxy_cache umamoe_embed_images;
    proxy_cache_lock on;
    proxy_cache_lock_timeout 10s;
    proxy_cache_background_update on;
    proxy_cache_use_stale error timeout updating http_500 http_502 http_503 http_504;
    proxy_cache_valid 200 5m;

    proxy_pass http://127.0.0.1:3108;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $http_cf_connecting_ip;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
    add_header X-Embed-Cache $upstream_cache_status always;
}

location / {
    error_page 418 = @umamoe_embeds_beta;
    if ($umamoe_use_embeds) {
        return 418;
    }

    try_files $uri $uri/ /index.html;
}

location @umamoe_embeds_beta {
    limit_req zone=umamoe_embed_html burst=30 nodelay;

    proxy_pass http://127.0.0.1:3108;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $http_cf_connecting_ip;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
    proxy_cache off;
}
```

For production, use the same locations but point them at `http://127.0.0.1:3008` and rename the named location if desired.

## Deployment

The workflow in `.github/workflows/docker-deploy.yml` follows the same image-archive deploy pattern as the other uma.moe services. On pull requests it builds the Docker image only. On pushes to `main` or `master`, and on manual `workflow_dispatch`, it deploys beta first and deploys production only after beta succeeds.

It reuses the shared GitHub environment/repository secrets:

- `DEPLOY_HOST`
- `DEPLOY_PORT` optional, defaults to `22`
- `DEPLOY_SSH_KEY`
- `DEPLOY_KNOWN_HOSTS`

Remote runtime env files are required and are not overwritten by the workflow:

- Beta: `/opt/umamoe-embeds-beta/env`
- Production: `/opt/umamoe-embeds/env`

Each env file must set `UMAMOE_FRONTEND_ORIGIN` to the static Angular shell origin for that environment. Keep optional runtime-only values there too, such as `UMAMOE_RESOURCES_API_TOKEN`, `UMAMOE_EMBEDS_BOT_USER_AGENT_TOKENS`, and `RUST_LOG`.

The workflow injects the environment-specific public and internal service URLs at container start:

- Beta: `UMAMOE_PUBLIC_BASE_URL=https://beta.uma.moe`, host port `3108`, container `umamoe-embeds-beta`, shared-network alias `umamoe-embeds-beta`
- Production: `UMAMOE_PUBLIC_BASE_URL=https://uma.moe`, host port `3008`, container `umamoe-embeds`, shared-network alias `umamoe-embeds`
- Beta dependencies: `http://umamoe-backend-beta`, `http://umamoe-search-beta:3202`, `http://umamoe-resources-beta:3204/resources`
- Production dependencies: `http://umamoe-backend`, `http://umamoe-search:3202`, `http://umamoe-resources:3204/resources`

Public exposure still depends on nginx routing the intended embed traffic to `127.0.0.1:3108` for beta and `127.0.0.1:3008` for production.

`UMAMOE_FRONTEND_ORIGIN` should not point at a public hostname that routes back through the embed service for normal SPA requests, or the fallback proxy can loop. If nginx only sends bot/debug/embed-image traffic to this service, the public static shell URL is fine.

After beta deploys, test:

```bash
curl -A "Discordbot/2.0" https://beta.uma.moe/circles/772781438
curl "https://beta.uma.moe/database?trainer_id=540903147493&__embed=1"
curl https://beta.uma.moe/__embeds/images/circle/772781438.png --output circle.png
```

## Local Run

From this folder:

```bash
cargo run
```

With a local Angular dev server:

```bash
set UMAMOE_FRONTEND_ORIGIN=http://127.0.0.1:4200
set UMAMOE_API_BASE_URL=http://127.0.0.1:3001
set UMAMOE_SEARCH_BASE_URL=http://127.0.0.1:3002
set UMAMOE_RESOURCES_BASE_URL=http://127.0.0.1:3004/resources
cargo run
```

With the local Docker stack:

```bash
docker compose -f ../umamoe-backend/compose.local.yml up --build embeds
```

For rebuild-on-change while testing inside Docker:

```bash
docker compose -f ../umamoe-backend/compose.local.yml -f ./compose.watch.yml watch embeds
```

Then use:

```text
http://127.0.0.1:3008/database?trainer_id=540903147493&__embed=1
```
