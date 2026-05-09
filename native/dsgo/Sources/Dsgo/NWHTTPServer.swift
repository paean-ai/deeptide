import Foundation
import Network

struct RawHTTPRequest {
    let method: String
    let path: String
    let headers: [String: String]
    let body: Data
}

enum HTTPParse {
    /// Consumes one full HTTP/1.1 request from `buffer` if available (supports only Content-Length bodies).
    static func consumeOne(buffer: inout Data) -> RawHTTPRequest? {
        guard let sepRange = buffer.range(of: Data("\r\n\r\n".utf8)) else { return nil }
        let headerBytes = buffer[..<sepRange.lowerBound]
        let bodyStart = sepRange.upperBound
        guard let headerText = String(data: Data(headerBytes), encoding: .utf8) else { return nil }
        let lines = headerText.split(separator: "\r\n", omittingEmptySubsequences: false).map(String.init)
        guard let first = lines.first else { return nil }
        let fparts = first.split(separator: " ", omittingEmptySubsequences: true)
        guard fparts.count >= 2 else { return nil }
        let method = String(fparts[0])
        let path = String(fparts[1])

        var hdrs: [String: String] = [:]
        for line in lines.dropFirst() where !line.isEmpty {
            guard let idx = line.firstIndex(of: ":") else { continue }
            let k = String(line[..<idx]).trimmingCharacters(in: .whitespaces).lowercased()
            let v = String(line[line.index(after: idx)...]).trimmingCharacters(in: .whitespaces)
            hdrs[k] = v
        }

        let methodUpper = method.uppercased()
        if methodUpper == "GET" || methodUpper == "HEAD" {
            buffer.removeSubrange(..<bodyStart)
            return RawHTTPRequest(method: methodUpper, path: path, headers: hdrs, body: Data())
        }

        guard let clStr = hdrs["content-length"], let cl = Int(clStr), cl >= 0 else { return nil }
        let end = bodyStart + cl
        guard buffer.count >= end else { return nil }
        let body = buffer.subdata(in: bodyStart..<end)
        buffer.removeSubrange(..<end)
        return RawHTTPRequest(method: methodUpper, path: path, headers: hdrs, body: body)
    }
}

final class GatewayHTTPServer: @unchecked Sendable {
    let config: GatewayConfig
    let host: String
    let port: UInt16
    let maxStreamBytes: Int

    private var listener: NWListener?

    init(config: GatewayConfig, host: String, port: UInt16, maxStreamBytes: Int) {
        self.config = config
        self.host = host
        self.port = port
        self.maxStreamBytes = maxStreamBytes
    }

    func start() throws {
        guard let nwPort = NWEndpoint.Port(rawValue: port) else {
            throw URLError(.badURL)
        }
        let listener = try NWListener(using: .tcp, on: nwPort)
        _ = host
        listener.stateUpdateHandler = { state in
            if case let .failed(err) = state {
                fputs("dsgo: listener failed: \(err)\n", stderr)
            }
        }
        listener.newConnectionHandler = { [weak self] conn in
            guard let self else {
                conn.cancel()
                return
            }
            let q = DispatchQueue(label: "dsgo.conn")
            conn.start(queue: q)
            ConnectionReceiver(connection: conn, queue: q, config: self.config, maxStreamBytes: self.maxStreamBytes).begin()
        }
        listener.start(queue: .global(qos: .userInitiated))
        self.listener = listener
    }

    func stop() {
        listener?.cancel()
        listener = nil
    }

    fileprivate static func handle(request: RawHTTPRequest, connection: NWConnection, queue: DispatchQueue, config: GatewayConfig, maxStreamBytes: Int) async {
        let path = request.path.split(separator: "?", maxSplits: 1).first.map(String.init) ?? request.path

        if request.method == "GET", path == "/health" {
            await Self.send(
                status: 200,
                extraHeaders: [("Content-Type", "application/json")],
                body: Data(#"{"status":"ok","service":"dsgo"}"#.utf8),
                connection: connection,
                responseQueue: queue
            )
            return
        }

        if request.method == "GET", path == "/v1/models" {
            do {
                let body = try config.models.openAIModelsResponse()
                await Self.send(status: 200, extraHeaders: [("Content-Type", "application/json")], body: body, connection: connection, responseQueue: queue)
            } catch {
                await Self.send(status: 502, extraHeaders: [("Content-Type", "text/plain")], body: Data("\(error.localizedDescription)\n".utf8), connection: connection, responseQueue: queue)
            }
            return
        }

        if request.method == "GET", path.hasPrefix("/v1/models/") {
            let modelName = String(path.dropFirst("/v1/models/".count))
            if let model = config.models.model(named: modelName) {
                let body: [String: Any] = [
                    "id": model.id,
                    "object": "model",
                    "created": 0,
                    "owned_by": "dsgo",
                    "backend": model.backend.rawValue,
                ]
                do {
                    let data = try JSONSerialization.data(withJSONObject: body, options: [.sortedKeys])
                    await Self.send(status: 200, extraHeaders: [("Content-Type", "application/json")], body: data, connection: connection, responseQueue: queue)
                } catch {
                    await Self.send(status: 502, extraHeaders: [("Content-Type", "text/plain")], body: Data("\(error.localizedDescription)\n".utf8), connection: connection, responseQueue: queue)
                }
            } else {
                await Self.send(status: 404, extraHeaders: [("Content-Type", "application/json")], body: Data(#"{"error":{"message":"model not found"}}"#.utf8), connection: connection, responseQueue: queue)
            }
            return
        }

        if request.method == "POST", path == "/v1/chat/completions" {
            await Self.forward(pathSuffix: "/v1/chat/completions", request: request, connection: connection, queue: queue, config: config, maxStreamBytes: maxStreamBytes)
            return
        }

        if request.method == "POST", path == "/v1/completions" {
            await Self.forward(pathSuffix: "/v1/completions", request: request, connection: connection, queue: queue, config: config, maxStreamBytes: maxStreamBytes)
            return
        }

        if request.method == "POST", path == "/v1/messages" {
            await Self.forward(pathSuffix: "/v1/messages", request: request, connection: connection, queue: queue, config: config, maxStreamBytes: maxStreamBytes)
            return
        }

        if request.method == "POST", path == "/v1/messages/count_tokens" {
            await Self.forward(pathSuffix: "/v1/messages/count_tokens", request: request, connection: connection, queue: queue, config: config, maxStreamBytes: maxStreamBytes)
            return
        }

        await Self.send(status: 404, extraHeaders: [("Content-Type", "text/plain")], body: Data("not found\n".utf8), connection: connection, responseQueue: queue)
    }

    private static func forward(pathSuffix: String, request: RawHTTPRequest, connection: NWConnection, queue: DispatchQueue, config: GatewayConfig, maxStreamBytes: Int) async {
        do {
            let json = try BodyParse.jsonObject(request.body)
            let base = config.upstream(for: json)
            if pathSuffix == "/v1/messages", base == config.llamaUpstream {
                try await Self.forwardAnthropicToOpenAI(
                    json: json,
                    request: request,
                    connection: connection,
                    queue: queue,
                    config: config
                )
                return
            }
            guard let url = URL(string: base + pathSuffix) else {
                await Self.send(status: 400, extraHeaders: [("Content-Type", "text/plain")], body: Data("bad upstream URL\n".utf8), connection: connection, responseQueue: queue)
                return
            }
            let snap = HTTPHeadersSnapshot(lowerHeaderDict: request.headers)
            let stream = json["stream"] as? Bool ?? false
            let fr: ForwardResult
            if stream {
                try await HttpForwarder.postStreamPassthrough(
                    to: url,
                    body: request.body,
                    incoming: snap,
                    maxBytes: maxStreamBytes,
                    onResponse: { status, headers in
                        await Self.sendStreamingHeaders(status: status, upstreamHeaders: headers, connection: connection, responseQueue: queue)
                    },
                    onChunk: { chunk in
                        await Self.sendRaw(chunk, connection: connection, responseQueue: queue)
                    }
                )
                await Self.close(connection: connection, responseQueue: queue)
                return
            } else {
                fr = try await HttpForwarder.postJSON(to: url, body: request.body, incoming: snap)
            }
            await Self.send(status: fr.status, upstreamHeaders: fr.headers, body: fr.body, connection: connection, responseQueue: queue)
        } catch let e as ProxyError {
            let msg = Data("\(e.description)\n".utf8)
            await Self.send(status: e.httpStatus, extraHeaders: [("Content-Type", "text/plain")], body: msg, connection: connection, responseQueue: queue)
        } catch {
            let msg = Data("\(error.localizedDescription)\n".utf8)
            await Self.send(status: 502, extraHeaders: [("Content-Type", "text/plain")], body: msg, connection: connection, responseQueue: queue)
        }
    }

    private static func forwardAnthropicToOpenAI(
        json: [String: Any],
        request: RawHTTPRequest,
        connection: NWConnection,
        queue: DispatchQueue,
        config: GatewayConfig
    ) async throws {
        guard let url = URL(string: config.llamaUpstream + "/v1/chat/completions") else {
            await Self.send(status: 400, extraHeaders: [("Content-Type", "text/plain")], body: Data("bad upstream URL\n".utf8), connection: connection, responseQueue: queue)
            return
        }
        let openAIBody = try AnthropicBridge.openAIChatBody(from: json)
        let snap = HTTPHeadersSnapshot(lowerHeaderDict: request.headers)
        let upstream = try await HttpForwarder.postJSON(to: url, body: openAIBody, incoming: snap)
        guard (200...299).contains(upstream.status) else {
            await Self.send(status: upstream.status, upstreamHeaders: upstream.headers, body: upstream.body, connection: connection, responseQueue: queue)
            return
        }
        let model = json["model"] as? String ?? "deepseek-coder-v2-lite"
        let stream = json["stream"] as? Bool ?? false
        let converted = try AnthropicBridge.anthropicBody(fromOpenAI: upstream.body, model: model, stream: stream)
        await Self.send(status: 200, upstreamHeaders: converted.headers, body: converted.body, connection: connection, responseQueue: queue)
    }

    private static func send(
        status: Int,
        upstreamHeaders: [(String, String)] = [],
        extraHeaders: [(String, String)] = [],
        body: Data,
        connection: NWConnection,
        responseQueue: DispatchQueue
    ) async {
        var merged: [(String, String)] = []
        merged.append(contentsOf: upstreamHeaders)
        merged.append(contentsOf: extraHeaders)
        await Self.sendHTTP(status: status, headers: merged, body: body, connection: connection, responseQueue: responseQueue)
    }

    private static func sendHTTP(
        status: Int,
        headers: [(String, String)],
        body: Data,
        connection: NWConnection,
        responseQueue: DispatchQueue
    ) async {
        await withCheckedContinuation { (cont: CheckedContinuation<Void, Never>) in
            let item = DispatchWorkItem {
                var lines = "HTTP/1.1 \(status) \(reasonPhrase(status))\r\n"
                lines += "Connection: close\r\n"
                var seenContentType = false
                for (k, v) in headers {
                    if k.lowercased() == "content-length" { continue }
                    if k.lowercased() == "connection" { continue }
                    if k.lowercased() == "transfer-encoding" { continue }
                    if k.lowercased() == "content-type" { seenContentType = true }
                    lines += "\(k): \(v)\r\n"
                }
                if !seenContentType {
                    lines += "Content-Type: application/json\r\n"
                }
                lines += "Content-Length: \(body.count)\r\n\r\n"
                guard let head = lines.data(using: .utf8) else {
                    connection.cancel()
                    cont.resume()
                    return
                }
                connection.send(content: head, contentContext: .defaultMessage, isComplete: false, completion: .contentProcessed({ _ in
                    connection.send(content: body, contentContext: .defaultMessage, isComplete: true, completion: .contentProcessed({ _ in
                        connection.cancel()
                        cont.resume()
                    }))
                }))
            }
            responseQueue.async(execute: item)
        }
    }

    private static func sendStreamingHeaders(
        status: Int,
        upstreamHeaders: [(String, String)],
        connection: NWConnection,
        responseQueue: DispatchQueue
    ) async {
        var lines = "HTTP/1.1 \(status) \(reasonPhrase(status))\r\n"
        lines += "Connection: close\r\n"
        var seenContentType = false
        for (k, v) in upstreamHeaders {
            let lower = k.lowercased()
            if lower == "content-length" || lower == "connection" || lower == "transfer-encoding" { continue }
            if lower == "content-type" { seenContentType = true }
            lines += "\(k): \(v)\r\n"
        }
        if !seenContentType {
            lines += "Content-Type: text/event-stream\r\n"
        }
        lines += "\r\n"
        await sendRaw(Data(lines.utf8), connection: connection, responseQueue: responseQueue)
    }

    private static func sendRaw(
        _ data: Data,
        connection: NWConnection,
        responseQueue: DispatchQueue
    ) async {
        await withCheckedContinuation { (cont: CheckedContinuation<Void, Never>) in
            responseQueue.async {
                connection.send(content: data, contentContext: .defaultMessage, isComplete: false, completion: .contentProcessed { _ in
                    cont.resume()
                })
            }
        }
    }

    private static func close(connection: NWConnection, responseQueue: DispatchQueue) async {
        await withCheckedContinuation { (cont: CheckedContinuation<Void, Never>) in
            responseQueue.async {
                connection.send(content: nil, contentContext: .defaultMessage, isComplete: true, completion: .contentProcessed { _ in
                    connection.cancel()
                    cont.resume()
                })
            }
        }
    }

    private static func reasonPhrase(_ code: Int) -> String {
        switch code {
        case 200: return "OK"
        case 204: return "No Content"
        case 400: return "Bad Request"
        case 404: return "Not Found"
        case 413: return "Payload Too Large"
        case 502: return "Bad Gateway"
        default: return "Error"
        }
    }
}

private final class ConnectionReceiver: @unchecked Sendable {
    let connection: NWConnection
    let queue: DispatchQueue
    let config: GatewayConfig
    let maxStreamBytes: Int
    private var buffer = Data()
    /// Keeps the receiver alive until the connection is finished (NW callbacks do not retain `self`).
    private var keepAlive: ConnectionReceiver?

    init(connection: NWConnection, queue: DispatchQueue, config: GatewayConfig, maxStreamBytes: Int) {
        self.connection = connection
        self.queue = queue
        self.config = config
        self.maxStreamBytes = maxStreamBytes
    }

    func begin() {
        keepAlive = self
        receiveMore()
    }

    private func end() {
        keepAlive = nil
    }

    private func receiveMore() {
        connection.receive(minimumIncompleteLength: 1, maximumLength: 8 * 1024 * 1024) { [weak self] data, _, isComplete, error in
            guard let self else { return }
            if error != nil {
                self.connection.cancel()
                self.end()
                return
            }
            if let data { self.buffer.append(data) }
            while let req = HTTPParse.consumeOne(buffer: &self.buffer) {
                Task {
                    await GatewayHTTPServer.handle(
                        request: req,
                        connection: self.connection,
                        queue: self.queue,
                        config: self.config,
                        maxStreamBytes: self.maxStreamBytes
                    )
                }
                self.end()
                return
            }
            if isComplete {
                self.connection.cancel()
                self.end()
                return
            }
            self.receiveMore()
        }
    }
}
