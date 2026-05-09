import Foundation

enum AnthropicBridge {
    static func openAIChatBody(from anthropic: [String: Any]) throws -> Data {
        let model = anthropic["model"] as? String ?? "deepseek-coder-v2-lite"
        let maxTokens = anthropic["max_tokens"] as? Int ?? 1024
        let temperature = anthropic["temperature"]

        var messages: [[String: Any]] = []
        let system = systemText(anthropic["system"])
        if !system.isEmpty {
            messages.append(["role": "system", "content": system])
        }

        if let inputMessages = anthropic["messages"] as? [[String: Any]] {
            for msg in inputMessages {
                guard let role = msg["role"] as? String else { continue }
                let content = contentText(msg["content"])
                if !content.isEmpty {
                    messages.append(["role": role == "assistant" ? "assistant" : "user", "content": content])
                }
            }
        }

        var out: [String: Any] = [
            "model": model,
            "messages": messages,
            "max_tokens": maxTokens,
            "stream": false,
        ]
        if let temperature { out["temperature"] = temperature }

        if let tools = anthropic["tools"] as? [[String: Any]], !tools.isEmpty {
            out["tools"] = tools.map { tool in
                [
                    "type": "function",
                    "function": [
                        "name": tool["name"] as? String ?? "tool",
                        "description": tool["description"] as? String ?? "",
                        "parameters": tool["input_schema"] as? [String: Any] ?? ["type": "object"],
                    ],
                ]
            }
        }

        return try JSONSerialization.data(withJSONObject: out, options: [])
    }

    static func anthropicBody(fromOpenAI data: Data, model: String, stream: Bool) throws -> (headers: [(String, String)], body: Data) {
        let obj = try JSONSerialization.jsonObject(with: data, options: [])
        guard let root = obj as? [String: Any] else {
            throw ProxyError.invalidJSON("OpenAI response must be a JSON object")
        }
        let id = root["id"] as? String ?? "msg_dsgo_\(UUID().uuidString.replacingOccurrences(of: "-", with: ""))"
        let choices = root["choices"] as? [[String: Any]]
        let first = choices?.first
        let message = first?["message"] as? [String: Any]
        let text = message?["content"] as? String ?? first?["text"] as? String ?? ""
        let stopReason = anthropicStopReason(first?["finish_reason"] as? String)
        let usage = anthropicUsage(root["usage"] as? [String: Any])

        if stream {
            let body = try anthropicSSE(id: id, model: model, text: text, stopReason: stopReason, usage: usage)
            return ([("Content-Type", "text/event-stream")], body)
        }

        let response: [String: Any] = [
            "id": id,
            "type": "message",
            "role": "assistant",
            "model": model,
            "content": [["type": "text", "text": text]],
            "stop_reason": stopReason,
            "stop_sequence": NSNull(),
            "usage": usage,
        ]
        return ([("Content-Type", "application/json")], try JSONSerialization.data(withJSONObject: response, options: []))
    }

    private static func systemText(_ value: Any?) -> String {
        if let s = value as? String { return s }
        if let blocks = value as? [[String: Any]] {
            return blocks.compactMap { $0["text"] as? String }.joined(separator: "\n\n")
        }
        return ""
    }

    private static func contentText(_ value: Any?) -> String {
        if let s = value as? String { return s }
        guard let blocks = value as? [[String: Any]] else { return "" }
        var parts: [String] = []
        for block in blocks {
            switch block["type"] as? String {
            case "text":
                if let text = block["text"] as? String { parts.append(text) }
            case "tool_result":
                if let s = block["content"] as? String {
                    parts.append("Tool result:\n\(s)")
                } else if let nested = block["content"] as? [[String: Any]] {
                    let text = nested.compactMap { $0["text"] as? String }.joined(separator: "\n")
                    if !text.isEmpty { parts.append("Tool result:\n\(text)") }
                }
            case "tool_use":
                if let name = block["name"] as? String {
                    parts.append("Tool request: \(name)")
                }
            default:
                continue
            }
        }
        return parts.joined(separator: "\n\n")
    }

    private static func anthropicStopReason(_ finish: String?) -> String {
        switch finish {
        case "length": return "max_tokens"
        case "tool_calls": return "tool_use"
        default: return "end_turn"
        }
    }

    private static func anthropicUsage(_ usage: [String: Any]?) -> [String: Any] {
        [
            "input_tokens": usage?["prompt_tokens"] as? Int ?? 0,
            "output_tokens": usage?["completion_tokens"] as? Int ?? 0,
        ]
    }

    private static func anthropicSSE(id: String, model: String, text: String, stopReason: String, usage: [String: Any]) throws -> Data {
        var chunks: [String] = []
        try chunks.append(event("message_start", [
            "type": "message_start",
            "message": [
                "id": id,
                "type": "message",
                "role": "assistant",
                "model": model,
                "content": [],
                "stop_reason": NSNull(),
                "stop_sequence": NSNull(),
                "usage": ["input_tokens": usage["input_tokens"] as? Int ?? 0, "output_tokens": 0],
            ],
        ]))
        try chunks.append(event("content_block_start", [
            "type": "content_block_start",
            "index": 0,
            "content_block": ["type": "text", "text": ""],
        ]))
        if !text.isEmpty {
            try chunks.append(event("content_block_delta", [
                "type": "content_block_delta",
                "index": 0,
                "delta": ["type": "text_delta", "text": text],
            ]))
        }
        try chunks.append(event("content_block_stop", ["type": "content_block_stop", "index": 0]))
        try chunks.append(event("message_delta", [
            "type": "message_delta",
            "delta": ["stop_reason": stopReason, "stop_sequence": NSNull()],
            "usage": usage,
        ]))
        try chunks.append(event("message_stop", ["type": "message_stop"]))
        return Data(chunks.joined().utf8)
    }

    private static func event(_ name: String, _ payload: [String: Any]) throws -> String {
        let data = try JSONSerialization.data(withJSONObject: payload, options: [])
        let json = String(data: data, encoding: .utf8) ?? "{}"
        return "event: \(name)\ndata: \(json)\n\n"
    }
}
