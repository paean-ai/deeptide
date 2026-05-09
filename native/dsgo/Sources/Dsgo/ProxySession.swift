import CFNetwork
import Foundation

/// Local upstream calls must **not** pick up macOS system HTTP proxies (same issue as Python `trust_env=False`).
enum ProxySession {
    static let shared: URLSession = {
        let config = URLSessionConfiguration.ephemeral
        config.timeoutIntervalForRequest = 600
        config.timeoutIntervalForResource = 600
        config.httpShouldSetCookies = false
        config.requestCachePolicy = .reloadIgnoringLocalCacheData
        config.connectionProxyDictionary = [
            kCFNetworkProxiesHTTPEnable as String: false,
            kCFNetworkProxiesHTTPSEnable as String: false,
            kCFNetworkProxiesSOCKSEnable as String: false,
        ]
        return URLSession(configuration: config)
    }()
}
