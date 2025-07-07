# Ratcrates



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
