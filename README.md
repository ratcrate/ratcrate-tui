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
