import Foundation

enum HttpForwarder {
    static func buildForwardHeaders(from incoming: HTTPHeadersSnapshot) -> [(String, String)] {
        var out: [(String, String)] = []
        if let ct = incoming.contentType {
            out.append(("Content-Type", ct))
        } else {
            out.append(("Content-Type", "application/json"))
        }
        if let v = incoming.anthropicVersion {
            out.append(("anthropic-version", v))
        } else {
            out.append(("anthropic-version", "2023-06-01"))
        }
        if let k = incoming.apiKey { out.append(("x-api-key", k)) }
        if let a = incoming.authorization { out.append(("Authorization", a)) }
        if let b = incoming.anthropicBeta { out.append(("anthropic-beta", b)) }
        return out
    }

    static func postJSON(
        to absoluteURL: URL,
        body: Data,
        incoming: HTTPHeadersSnapshot
    ) async throws -> ForwardResult {
        var req = URLRequest(url: absoluteURL)
        req.httpMethod = "POST"
        req.httpBody = body
        for (n, v) in buildForwardHeaders(from: incoming) {
            req.setValue(v, forHTTPHeaderField: n)
        }
        let (data, resp) = try await ProxySession.shared.data(for: req)
        guard let http = resp as? HTTPURLResponse else {
            throw ProxyError.upstream(502, "non-HTTP response")
        }
        let pairs = headerPairs(http)
        return ForwardResult(status: http.statusCode, headers: HopByHop.filterResponseHeaders(pairs), body: data)
    }

    /// Reads a streaming upstream fully into memory (fine for moderate SSE sessions). Increase cap if needed.
    static func postStreamBuffered(
        to absoluteURL: URL,
        body: Data,
        incoming: HTTPHeadersSnapshot,
        maxBytes: Int
    ) async throws -> ForwardResult {
        var req = URLRequest(url: absoluteURL)
        req.httpMethod = "POST"
        req.httpBody = body
        for (n, v) in buildForwardHeaders(from: incoming) {
            req.setValue(v, forHTTPHeaderField: n)
        }
        let (bytes, resp) = try await ProxySession.shared.bytes(for: req)
        guard let http = resp as? HTTPURLResponse else {
            throw ProxyError.upstream(502, "non-HTTP response")
        }
        let pairs = headerPairs(http)
        var buf = Data()
        buf.reserveCapacity(min(maxBytes, 65536))
        for try await byte in bytes {
            buf.append(byte)
            if buf.count > maxBytes {
                throw ProxyError.upstream(413, "anthropic stream exceeds DSGO_MAX_STREAM_BYTES=\(maxBytes)")
            }
        }
        return ForwardResult(status: http.statusCode, headers: HopByHop.filterResponseHeaders(pairs), body: buf)
    }

    static func postStreamPassthrough(
        to absoluteURL: URL,
        body: Data,
        incoming: HTTPHeadersSnapshot,
        maxBytes: Int,
        onResponse: (Int, [(String, String)]) async throws -> Void,
        onChunk: (Data) async throws -> Void
    ) async throws {
        var req = URLRequest(url: absoluteURL)
        req.httpMethod = "POST"
        req.httpBody = body
        for (n, v) in buildForwardHeaders(from: incoming) {
            req.setValue(v, forHTTPHeaderField: n)
        }
        let (bytes, resp) = try await ProxySession.shared.bytes(for: req)
        guard let http = resp as? HTTPURLResponse else {
            throw ProxyError.upstream(502, "non-HTTP response")
        }
        try await onResponse(http.statusCode, HopByHop.filterResponseHeaders(headerPairs(http)))

        var count = 0
        var chunk = Data()
        chunk.reserveCapacity(16 * 1024)
        for try await byte in bytes {
            chunk.append(byte)
            count += 1
            if count > maxBytes {
                throw ProxyError.upstream(413, "stream exceeds DSGO_MAX_STREAM_BYTES=\(maxBytes)")
            }
            if chunk.count >= 16 * 1024 {
                try await onChunk(chunk)
                chunk.removeAll(keepingCapacity: true)
            }
        }
        if !chunk.isEmpty {
            try await onChunk(chunk)
        }
    }

    private static func headerPairs(_ http: HTTPURLResponse) -> [(String, String)] {
        http.allHeaderFields.compactMap { k, v -> (String, String)? in
            guard let ks = k as? String, let vs = v as? String else { return nil }
            return (ks, vs)
        }
    }
}

struct HTTPHeadersSnapshot: Sendable {
    var contentType: String?
    var anthropicVersion: String?
    var apiKey: String?
    var authorization: String?
    var anthropicBeta: String?

    init(lowerHeaderDict: [String: String]) {
        func first(_ names: [String]) -> String? {
            for n in names {
                if let v = lowerHeaderDict[n.lowercased()] { return v }
            }
            return nil
        }
        contentType = first(["content-type"])
        anthropicVersion = first(["anthropic-version"])
        apiKey = first(["x-api-key"])
        authorization = first(["authorization"])
        anthropicBeta = first(["anthropic-beta"])
    }
}

struct ForwardResult: Sendable {
    let status: Int
    let headers: [(String, String)]
    let body: Data
}
