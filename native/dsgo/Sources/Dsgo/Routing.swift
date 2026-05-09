import Foundation

struct GatewayConfig: Sendable {
    var llamaUpstream: String
    var ds4Upstream: String
    var models: ModelRegistry

    init(llamaUpstream: String, ds4Upstream: String, models: ModelRegistry) {
        self.llamaUpstream = Self.normalizeBase(llamaUpstream)
        self.ds4Upstream = Self.normalizeBase(ds4Upstream)
        self.models = models
    }

    private static func normalizeBase(_ s: String) -> String {
        var t = s.trimmingCharacters(in: .whitespacesAndNewlines)
        while t.hasSuffix("/") { t.removeLast() }
        return t
    }

    func upstream(for body: [String: Any]) -> String {
        let modelName = body["model"] as? String
        switch models.model(named: modelName)?.backend {
        case .ds4:
            return ds4Upstream
        case .llama:
            return llamaUpstream
        case nil:
            if modelName?.hasPrefix("ds4/") == true { return ds4Upstream }
            return llamaUpstream
        }
    }

    func upstream(for modelName: String?) -> String {
        switch models.model(named: modelName)?.backend {
        case .ds4:
            return ds4Upstream
        case .llama:
            return llamaUpstream
        case nil:
            if modelName?.hasPrefix("ds4/") == true { return ds4Upstream }
            return llamaUpstream
        }
    }
}

enum BodyParse {
    static func jsonObject(_ data: Data) throws -> [String: Any] {
        let obj = try JSONSerialization.jsonObject(with: data, options: [])
        guard let dict = obj as? [String: Any] else {
            throw ProxyError.invalidJSON("Body must be a JSON object")
        }
        return dict
    }
}

enum ProxyError: Error, CustomStringConvertible {
    case invalidJSON(String)
    case upstream(Int, String)

    var description: String {
        switch self {
        case let .invalidJSON(s): return s
        case let .upstream(code, msg): return "upstream \(code): \(msg)"
        }
    }

    var httpStatus: Int {
        switch self {
        case .invalidJSON: return 400
        case .upstream(let code, _): return code
        }
    }
}

enum HopByHop {
    static let names: Set<String> = [
        "connection", "keep-alive", "proxy-authenticate", "proxy-authorization",
        "te", "trailers", "transfer-encoding", "upgrade",
    ]

    static func filterResponseHeaders(_ headers: [(String, String)]) -> [(String, String)] {
        headers.filter { !names.contains($0.0.lowercased()) }
    }
}
