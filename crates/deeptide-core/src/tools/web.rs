//! Web tools: fetch a URL as readable text (WebFetch) and search the web (WebSearch).
//!
//! Shared infrastructure lives in the parent module (reached via `use super::*`).

use super::*;

#[derive(Debug, Default, Clone, Copy)]
pub struct WebFetchTool;

impl Tool for WebFetchTool {
    fn name(&self) -> &'static str {
        "WebFetch"
    }

    fn description(&self) -> &'static str {
        "Fetch web content over HTTP or HTTPS and return readable text."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let Some(url_value) = input.get("url").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("url is required");
        };
        if url_value.trim().is_empty() {
            return ToolResult::error("url is required");
        }

        let requested_url = match Url::parse(url_value) {
            Ok(url) if matches!(url.scheme(), "http" | "https") => url,
            _ => return ToolResult::error(format!("Invalid URL: {url_value}")),
        };

        fetch_web_content(&requested_url)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct WebSearchTool;

impl WebSearchTool {
    pub fn call_with_environment(
        &self,
        input: serde_json::Value,
        env: &BTreeMap<String, String>,
    ) -> ToolResult {
        web_search_with_environment(input, env)
    }
}

impl Tool for WebSearchTool {
    fn name(&self) -> &'static str {
        "WebSearch"
    }

    fn description(&self) -> &'static str {
        "Search the web using optional Brave Search or Serper credentials."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let env = std::env::vars().collect::<BTreeMap<_, _>>();
        self.call_with_environment(input, &env)
    }
}

fn web_search_with_environment(
    input: serde_json::Value,
    env: &BTreeMap<String, String>,
) -> ToolResult {
    let Some(query) = input.get("query").and_then(serde_json::Value::as_str) else {
        return ToolResult::error("Missing query parameter");
    };
    if query.chars().count() < 2 {
        return ToolResult::error("query must be at least 2 characters");
    }
    if input.get("allowed_domains").is_some() && input.get("blocked_domains").is_some() {
        return ToolResult::error("Cannot specify both allowed_domains and blocked_domains");
    }

    let allowed_domains = extract_string_array(input.get("allowed_domains"));
    let blocked_domains = extract_string_array(input.get("blocked_domains"));
    let mut provider_errors = Vec::new();

    if let Some(api_key) = env
        .get("BRAVE_SEARCH_API_KEY")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        match search_brave(query, api_key, &allowed_domains, &blocked_domains) {
            Ok(results) => return ToolResult::text(results),
            Err(error) => provider_errors.push(format!("Brave Search: {error}")),
        }
    }

    if let Some(api_key) = env
        .get("SERPER_API_KEY")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        match search_serper(query, api_key, &allowed_domains, &blocked_domains) {
            Ok(results) => return ToolResult::text(results),
            Err(error) => provider_errors.push(format!("Serper: {error}")),
        }
    }

    let encoded = encode_query_component(query);

    if !provider_errors.is_empty() {
        return ToolResult::error(format!(
            "Configured WebSearch backend(s) failed:\n  - {}\n\nCheck the configured endpoint/key, or provide specific URLs and use WebFetch. You can also fetch a search-results page directly:\n  - https://html.duckduckgo.com/html/?q={encoded}\n  - https://www.google.com/search?q={encoded}",
            provider_errors.join("\n  - ")
        ));
    }

    ToolResult::error(format!(
        "WebSearch has no configured search backend. Optional backends:\n  export BRAVE_SEARCH_API_KEY=<key>    # Brave Search API\n  export SERPER_API_KEY=<key>          # Serper / Google results\n\nFor /deep-seek, this is not fatal: the model can propose likely official/canonical URLs from its built-in knowledge and verify them with WebFetch. To discover sources without a backend, fetch a search-results page directly:\n  - https://html.duckduckgo.com/html/?q={encoded}\n  - https://www.google.com/search?q={encoded}"
    ))
}

#[derive(Debug, Deserialize)]
struct BraveSearchResponse {
    web: Option<BraveWebResults>,
}

#[derive(Debug, Deserialize)]
struct BraveWebResults {
    results: Vec<BraveWebResult>,
}

#[derive(Debug, Clone, Deserialize)]
struct BraveWebResult {
    title: String,
    url: String,
    description: Option<String>,
    age: Option<String>,
}

fn search_brave(
    query: &str,
    api_key: &str,
    allowed_domains: &[String],
    blocked_domains: &[String],
) -> Result<String, String> {
    let mut url =
        Url::parse("https://api.search.brave.com/res/v1/web/search").map_err(|e| e.to_string())?;
    url.query_pairs_mut()
        .append_pair("q", query)
        .append_pair("count", "10")
        .append_pair("text_decorations", "false");

    let client = web_search_client()?;
    let response = client
        .get(url)
        .header("X-Subscription-Token", api_key)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "Brave Search failed with HTTP {}",
            response.status()
        ));
    }

    let parsed = response
        .json::<BraveSearchResponse>()
        .map_err(|error| error.to_string())?;
    let results = parsed.web.map(|web| web.results).unwrap_or_default();
    let results = apply_domain_filters(
        results
            .into_iter()
            .map(|result| (result.url.clone(), result)),
        allowed_domains,
        blocked_domains,
    );

    Ok(format_brave_results(query, &results))
}

fn format_brave_results(query: &str, results: &[BraveWebResult]) -> String {
    if results.is_empty() {
        return format!("No results found for: \"{query}\"");
    }

    let mut lines = vec![format!(
        "Web search results for: \"{query}\" (Brave Search)"
    )];
    for (index, result) in results.iter().take(8).enumerate() {
        lines.push(String::new());
        lines.push(format!("{}. {}", index + 1, result.title));
        lines.push(format!("   URL: {}", result.url));
        if let Some(description) = result
            .description
            .as_ref()
            .filter(|value| !value.is_empty())
        {
            lines.push(format!("   {description}"));
        }
        if let Some(age) = result.age.as_ref().filter(|value| !value.is_empty()) {
            lines.push(format!("   {age}"));
        }
    }
    lines.push(String::new());
    lines.push(String::from(
        "Use WebFetch on any URL above to read the full page content.",
    ));
    lines.join("\n")
}

#[derive(Debug, Deserialize)]
struct SerperResponse {
    organic: Option<Vec<SerperOrganic>>,
}

#[derive(Debug, Clone, Deserialize)]
struct SerperOrganic {
    title: String,
    link: String,
    snippet: Option<String>,
    date: Option<String>,
}

fn search_serper(
    query: &str,
    api_key: &str,
    allowed_domains: &[String],
    blocked_domains: &[String],
) -> Result<String, String> {
    let client = web_search_client()?;
    let response = client
        .post("https://google.serper.dev/search")
        .header("X-API-KEY", api_key)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(&serde_json::json!({"q": query, "num": 10}))
        .send()
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "Serper search failed with HTTP {}",
            response.status()
        ));
    }

    let parsed = response
        .json::<SerperResponse>()
        .map_err(|error| error.to_string())?;
    let results = apply_domain_filters(
        parsed
            .organic
            .unwrap_or_default()
            .into_iter()
            .map(|result| (result.link.clone(), result)),
        allowed_domains,
        blocked_domains,
    );

    Ok(format_serper_results(query, &results))
}

fn format_serper_results(query: &str, results: &[SerperOrganic]) -> String {
    if results.is_empty() {
        return format!("No results found for: \"{query}\"");
    }

    let mut lines = vec![format!(
        "Web search results for: \"{query}\" (Google via Serper)"
    )];
    for (index, result) in results.iter().take(8).enumerate() {
        lines.push(String::new());
        lines.push(format!("{}. {}", index + 1, result.title));
        lines.push(format!("   URL: {}", result.link));
        if let Some(snippet) = result.snippet.as_ref().filter(|value| !value.is_empty()) {
            lines.push(format!("   {snippet}"));
        }
        if let Some(date) = result.date.as_ref().filter(|value| !value.is_empty()) {
            lines.push(format!("   {date}"));
        }
    }
    lines.push(String::new());
    lines.push(String::from(
        "Use WebFetch on any URL above to read the full page content.",
    ));
    lines.join("\n")
}

fn web_search_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|error| error.to_string())
}

fn apply_domain_filters<T>(
    items: impl IntoIterator<Item = (String, T)>,
    allowed_domains: &[String],
    blocked_domains: &[String],
) -> Vec<T> {
    items
        .into_iter()
        .filter(|(url, _)| domain_allowed(url, allowed_domains, blocked_domains))
        .take(8)
        .map(|(_, value)| value)
        .collect()
}

fn domain_allowed(url: &str, allowed_domains: &[String], blocked_domains: &[String]) -> bool {
    let Some(host) = Url::parse(url)
        .ok()
        .and_then(|url| url.host_str().map(ToOwned::to_owned))
    else {
        return true;
    };
    if !allowed_domains.is_empty() {
        return allowed_domains
            .iter()
            .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")));
    }
    if !blocked_domains.is_empty() {
        return !blocked_domains
            .iter()
            .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")));
    }
    true
}

fn extract_string_array(value: Option<&serde_json::Value>) -> Vec<String> {
    value
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// URL-encode `query` as a `q=` query-parameter value, so the WebFetch fallback
/// URLs below point at a real search-results page. Falls back to the raw query
/// if URL construction somehow fails (never panics).
fn encode_query_component(query: &str) -> String {
    let Ok(mut url) = Url::parse("https://example.invalid/") else {
        return query.to_owned();
    };
    url.query_pairs_mut().append_pair("q", query);
    url.query()
        .and_then(|query| query.strip_prefix("q="))
        .unwrap_or(query)
        .to_owned()
}

fn fetch_web_content(url: &Url) -> ToolResult {
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 \
             (KHTML, like Gecko) Deeptide/1.0 Safari/537.36",
        )
        .build()
    {
        Ok(client) => client,
        Err(error) => return ToolResult::error(format!("Failed to create HTTP client: {error}")),
    };

    let response = match client
        .get(url.clone())
        .header(
            reqwest::header::ACCEPT,
            "text/html,application/xhtml+xml,application/xml;q=0.9,text/plain;q=0.8,*/*;q=0.5",
        )
        .header(reqwest::header::ACCEPT_LANGUAGE, "en-US,en;q=0.9")
        .send()
    {
        Ok(response) => response,
        Err(error) => return ToolResult::error(format!("Failed to fetch URL: {error}")),
    };

    let status = response.status();
    let final_url = response.url().clone();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unknown")
        .to_owned();
    let selected_headers = selected_response_headers(&response);
    let bytes = match response.bytes() {
        Ok(bytes) => bytes,
        Err(error) => return ToolResult::error(format!("Failed to read response body: {error}")),
    };
    let size = bytes.len();

    let body = decode_response_body(&bytes);
    let is_http_error = !status.is_success();
    let mut output = response_header(
        status.as_u16(),
        size,
        url.as_str(),
        final_url.as_str(),
        &content_type,
        &selected_headers,
    );

    let Some(body) = body else {
        output.push_str("\n\nCould not decode response body (binary content).");
        return if is_http_error {
            ToolResult::error(output)
        } else {
            ToolResult::text(output)
        };
    };

    if is_http_error {
        output.push_str("\n\n--- HTTP diagnostics ---\n");
        output.push_str(&http_diagnostic(
            status.as_u16(),
            &selected_headers,
            final_url.as_str(),
            url.as_str(),
        ));
    }

    let lower_content_type = content_type.to_lowercase();
    let is_html_like = lower_content_type.contains("html") || body.to_lowercase().contains("<html");
    let mut text = if is_html_like {
        html_to_text(&body, Some(&final_url))
    } else {
        decode_entities(&body)
    };
    text = text.trim().to_owned();
    let total_chars = text.chars().count();
    if total_chars > 50_000 {
        text = format!(
            "{}\n\n[Content truncated: {total_chars} total chars; fetch a narrower URL or linked resource if needed]",
            truncate_chars(&text, 50_000)
        );
    }

    output.push_str("\n\n");
    output.push_str(&text);

    if is_http_error {
        ToolResult::error(output)
    } else {
        ToolResult::text(output)
    }
}

fn selected_response_headers(response: &reqwest::blocking::Response) -> Vec<(String, String)> {
    ["Location", "Retry-After", "WWW-Authenticate", "Server"]
        .into_iter()
        .filter_map(|name| {
            response
                .headers()
                .get(name)
                .and_then(|value| value.to_str().ok())
                .filter(|value| !value.is_empty())
                .map(|value| (name.to_owned(), value.to_owned()))
        })
        .collect()
}

fn decode_response_body(bytes: &[u8]) -> Option<String> {
    String::from_utf8(bytes.to_vec()).ok().or_else(|| {
        if bytes.contains(&0) {
            None
        } else {
            Some(bytes.iter().map(|byte| char::from(*byte)).collect())
        }
    })
}

fn response_header(
    code: u16,
    size: usize,
    requested_url: &str,
    final_url: &str,
    content_type: &str,
    selected_headers: &[(String, String)],
) -> String {
    let mut lines = vec![
        format!("HTTP {code} | {}", format_byte_count(size)),
        format!("URL: {requested_url}"),
    ];
    if final_url != requested_url {
        lines.push(format!("Final URL: {final_url}"));
    }
    lines.push(format!("Content-Type: {content_type}"));
    lines.extend(
        selected_headers
            .iter()
            .map(|(name, value)| format!("{name}: {value}")),
    );
    lines.join("\n")
}

fn http_diagnostic(
    code: u16,
    selected_headers: &[(String, String)],
    final_url: &str,
    requested_url: &str,
) -> String {
    let mut lines = Vec::new();
    if final_url != requested_url {
        lines.push(String::from(
            "Request followed redirects from the requested URL; verify the final URL when diagnosing.",
        ));
    }

    match code {
        400 => lines.push(String::from("400 Bad Request: inspect query encoding, required parameters, and whether the endpoint expects a different method or content type.")),
        401 => lines.push(String::from("401 Unauthorized: authentication is missing, expired, or not accepted by this endpoint.")),
        403 => lines.push(String::from("403 Forbidden: access may require login, a token, different permissions, or the site may be blocking automated clients.")),
        404 => lines.push(String::from("404 Not Found: verify the path, slug, version, and redirects; the host is reachable but this resource was not served.")),
        408 => lines.push(String::from("408 Request Timeout: retry later and check whether the server expects a smaller or more specific request.")),
        409 => lines.push(String::from("409 Conflict: the request reached the endpoint but conflicts with current resource state.")),
        410 => lines.push(String::from("410 Gone: the resource has been intentionally removed or archived.")),
        429 => {
            let retry = selected_headers
                .iter()
                .find(|(name, _)| name == "Retry-After")
                .map(|(_, value)| format!(" Retry-After: {value}."))
                .unwrap_or_default();
            lines.push(format!("429 Rate Limited: wait before retrying, reduce request frequency, or use an authenticated API.{retry}"));
        }
        500..=599 => lines.push(format!("{code} Server Error: the upstream service failed; capture the body, final URL, and relevant headers before retrying.")),
        _ => lines.push(format!("{code} HTTP status: use the response body plus headers above as the primary evidence.")),
    }

    lines.join("\n")
}

fn html_to_text(html: &str, base_url: Option<&Url>) -> String {
    let mut text = regex_replace(html, r"(?is)<!--.*?-->", "");
    text = regex_replace(&text, r"(?is)<script\b[^>]*>.*?</script>", "");
    text = regex_replace(&text, r"(?is)<style\b[^>]*>.*?</style>", "");
    text = replace_anchor_tags(&text, base_url);
    text = regex_replace(&text, r"(?i)<br\s*/?>", "\n");
    text = regex_replace(
        &text,
        r"(?i)</(p|div|section|article|header|footer|main|nav|tr|table|ul|ol|h[1-6])\s*>",
        "\n",
    );
    text = regex_replace(&text, r"(?i)<li\b[^>]*>", "\n- ");
    text = regex_replace(&text, r"(?i)</(td|th)\s*>", "\t");
    text = regex_replace(&text, r"<[^>]+>", "");

    let decoded = decode_entities(&text);
    let decoded = regex_replace(&decoded, r"[ \t\u{00A0}]+", " ");
    let decoded = regex_replace(&decoded, r" *\n *", "\n");
    regex_replace(&decoded, r"\n{3,}", "\n\n")
}

fn replace_anchor_tags(html: &str, base_url: Option<&Url>) -> String {
    let Ok(regex) = regex::Regex::new(r#"(?is)<a\b([^>]*)>(.*?)</a>"#) else {
        return html.to_owned();
    };
    regex
        .replace_all(html, |captures: &regex::Captures<'_>| {
            let attrs = captures.get(1).map(|value| value.as_str()).unwrap_or("");
            let inner = captures.get(2).map(|value| value.as_str()).unwrap_or("");
            let label = regex_replace(inner, r"<[^>]+>", "");
            if let Some(href) = extract_href(attrs).filter(|href| !href.is_empty()) {
                let resolved = base_url
                    .and_then(|base| base.join(&href).ok())
                    .map(|url| url.to_string())
                    .unwrap_or(href);
                format!("{label} [{resolved}]")
            } else {
                label
            }
        })
        .into_owned()
}

fn extract_href(attrs: &str) -> Option<String> {
    let regex = regex::Regex::new(r#"(?is)\bhref\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))"#).ok()?;
    let captures = regex.captures(attrs)?;
    (1..=3)
        .find_map(|group| captures.get(group).map(|value| value.as_str()))
        .map(decode_entities)
}

fn decode_entities(text: &str) -> String {
    let mut output = String::new();
    let mut chars = text.char_indices().peekable();
    while let Some((index, ch)) = chars.next() {
        if ch != '&' {
            output.push(ch);
            continue;
        }

        let Some((semicolon_index, _)) = text[index..]
            .char_indices()
            .take_while(|(_, candidate)| *candidate != '\n')
            .find(|(_, candidate)| *candidate == ';')
        else {
            output.push(ch);
            continue;
        };
        if semicolon_index > 12 {
            output.push(ch);
            continue;
        }

        let entity = &text[index + 1..index + semicolon_index];
        if let Some(replacement) = decode_entity(entity) {
            output.push(replacement);
            while chars
                .peek()
                .is_some_and(|(next_index, _)| *next_index <= index + semicolon_index)
            {
                chars.next();
            }
        } else {
            output.push(ch);
        }
    }
    output
}

fn decode_entity(entity: &str) -> Option<char> {
    match entity {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" | "#39" => Some('\''),
        "nbsp" => Some(' '),
        value if value.starts_with("#x") || value.starts_with("#X") => {
            u32::from_str_radix(&value[2..], 16)
                .ok()
                .and_then(char::from_u32)
        }
        value if value.starts_with('#') => value[1..].parse::<u32>().ok().and_then(char::from_u32),
        _ => None,
    }
}

fn regex_replace(input: &str, pattern: &str, replacement: &str) -> String {
    regex::Regex::new(pattern)
        .map(|regex| regex.replace_all(input, replacement).into_owned())
        .unwrap_or_else(|_| input.to_owned())
}
