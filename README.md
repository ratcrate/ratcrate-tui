# Ratcrates

# Architecture 

```markdown
                    ┌─────────────────┐
                    │   Central DB    │
                    │   (GitHub repo  │
                    │   + JSON files) │
                    └─────────────────┘
                             │
            ┌────────────────┼────────────────┐
            │                │                │
    ┌───────▼────────┐ ┌─────▼─────┐ ┌───────▼────────┐
    │   CLI Tool     │ │ Web App   │ │ Neovim Plugin  │
    │   (Rust)       │ │ (Dioxus)  │ │ (Lua + Rust)   │
    └────────────────┘ └───────────┘ └────────────────┘

```


# Web Page

```markdown
┌─────────────────────────────────────────────────────────────┐
│                        RatCrate                             │
│                 Discover Terminal Apps                      │
├─────────────────────────────────────────────────────────────┤
│  [Search]  [Categories ▼]  [Sort: Popular ▼]  [Submit App]  │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  🔥 Featured This Week                                       │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐            │  
│  │ spotify-tui │ │    gitui    │ │   bottom    │            │
│  │ [preview]   │ │ [preview]   │ │ [preview]   │            │
│  └─────────────┘ └─────────────┘ └─────────────┘            │
│                                                             │
│  📱 Categories                                              │
│  • Media (12)     • Development (25)   • System (18)        │
│  • Games (8)      • Productivity (15)  • Network (9)        │
│                                                             │
│  📈 Trending                                                │
│  1. gitui - Fast terminal UI for git                       │
│  2. spotify-tui - Spotify client for terminal              │
│  3. bottom - System monitor                                │
└─────────────────────────────────────────────────────────────┘


┌─────────────────────────────────────────────────────────────┐
│  spotify-tui                                    ⭐ 8.2k     │
│  A Spotify client for the terminal                          │
│                                                             │
│  [📋 Copy Install] [🔗 GitHub] [📖 Docs] [🐛 Issues]          │
│                                                              │
│  📸 Screenshots                                              │
│  [Screenshot carousel]                                      │
│                                                             │
│  📦 Installation                                            │ 
│  cargo install spotify-tui                                  │
│                                                             │
│  📋 Details                                                 │
│  • Category: Media                                          │
│  • Downloads: 45k                                           │
│  • Last Updated: 2 days ago                                 │
│  • License: MIT                                             │
│                                                             │
│  💬 Community Reviews                                       │
│  ⭐⭐⭐⭐⭐ "Amazing! Works perfectly" - user123              │
└─────────────────────────────────────────────────────────────┘


```











# Technical Details


GITHUB API using curl 
```bash
$> curl -H "Accept: application/vnd.github.v3+json" \
  "https://api.github.com/search/repositories?q=ratatui+language:rust"

```
Crate Reverse API 

```bash
$> curl "https://crates.io/api/v1/crates/ratatui/reverse_dependencies"
```

Using GraphQL
```bash
$> curl -H "Authorization: bearer YOUR_TOKEN" \
  -X POST -d '{"query": "query { search(query: \"ratatui language:rust\", type: REPOSITORY, first: 100) { edges { node { ... on Repository { name description url } } } } }"}' \
  https://api.github.com/graphql
```


curl and jq
```bash
# to print output as is
$> curl "https://crates.io/api/v1/crates/ratatui/reverse_dependencies" | jq '.'

# To get the designed field, here's what we will use to run the command
# First get the .json file
$> curl "https://crates.io/api/v1/crates/ratatui/reverse_dependencies" | jq -r '.'  > crates_data.json
$> jq -r '.versions[] | {
    crate: .crate,
    last_updated: .updated_at,
    downloads: .downloads,
    description: .description,
    license: .license,
    published_by_login: .published_by.login,
    published_by_name: .published_by.name,
    published_by_github_url: .published_by.url
}' crates_data.json

# To avoid the downloads, let us use this command; We will use this command

$> curl "https://crates.io/api/v1/crates/ratatui/reverse_dependencies" | jq -r '.versions[] | {
    crate: .crate,
    last_updated: .updated_at,
    downloads: .downloads,
    description: .description,
    license: .license,
    published_by_login: .published_by.login,
    published_by_name: .published_by.name,
    published_by_github_url: .published_by.url
}'

```

This is the current output. 
```json
{
  "crate": "russh",
  "last_updated": "2025-07-01T21:47:44.037693Z",
  "downloads": 1154,
  "description": "A client and server SSH library.",
  "license": "Apache-2.0",
  "published_by_login": "Eugeny",
  "published_by_name": "Eugene",
  "published_by_github_url": "https://github.com/Eugeny"
}
```

I want to add few more to the above list
- [ ] category -  "media", "games", "productivity", "coding" etc
- [ ] manual_review: true - Initially it will be manually reviewed
- [ ] tags - ["music", "spotify", "streaming"],
- [ ] screenshots -  ["url1", "url2"] to be decided





















 


