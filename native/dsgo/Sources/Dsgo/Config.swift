import Foundation

enum BackendKind: String, Codable, Sendable {
    case ds4
    case llama
}

struct RegisteredModel: Codable, Sendable {
    var id: String
    var backend: BackendKind
    var aliases: [String]
    var contextWindow: Int?
    var maxOutputTokens: Int?
    var reasoning: Bool?

    init(
        id: String,
        backend: BackendKind,
        aliases: [String] = [],
        contextWindow: Int? = nil,
        maxOutputTokens: Int? = nil,
        reasoning: Bool? = nil
    ) {
        self.id = id
        self.backend = backend
        self.aliases = aliases
        self.contextWindow = contextWindow
        self.maxOutputTokens = maxOutputTokens
        self.reasoning = reasoning
    }
}

struct ModelRegistry: Sendable {
    var models: [RegisteredModel]
    private var byName: [String: RegisteredModel]

    init(models: [RegisteredModel]) {
        self.models = models
        var index: [String: RegisteredModel] = [:]
        for model in models {
            index[model.id] = model
            for alias in model.aliases {
                index[alias] = model
            }
        }
        self.byName = index
    }

    static func defaults(ds4Models: Set<String>) -> ModelRegistry {
        var models: [RegisteredModel] = [
            RegisteredModel(
                id: "deepseek-v4-flash",
                backend: .ds4,
                aliases: ["ds4/deepseek-v4-flash", "deepseek-reasoner", "deepseek-chat"],
                contextWindow: 100_000,
                maxOutputTokens: 384_000,
                reasoning: true
            ),
            RegisteredModel(
                id: "deepseek-coder-v2-lite",
                backend: .llama,
                aliases: ["llama/deepseek-coder-v2-lite"],
                contextWindow: 32_768,
                maxOutputTokens: 16_384,
                reasoning: false
            ),
        ]

        for id in ds4Models where !models.contains(where: { $0.id == id }) {
            models.append(RegisteredModel(id: id, backend: .ds4, aliases: ["ds4/\(id)"], reasoning: true))
        }
        return ModelRegistry(models: models)
    }

    func model(named name: String?) -> RegisteredModel? {
        guard let name, !name.isEmpty else { return nil }
        if let model = byName[name] { return model }
        if name.hasPrefix("ds4/") {
            return RegisteredModel(id: String(name.dropFirst(4)), backend: .ds4, aliases: [name], reasoning: true)
        }
        if name.hasPrefix("llama/") {
            return RegisteredModel(id: String(name.dropFirst(6)), backend: .llama, aliases: [name], reasoning: false)
        }
        return nil
    }

    func openAIModelsResponse() throws -> Data {
        let items = models.map { model -> [String: Any] in
            var item: [String: Any] = [
                "id": model.id,
                "object": "model",
                "created": 0,
                "owned_by": "dsgo",
            ]
            item["backend"] = model.backend.rawValue
            if let contextWindow = model.contextWindow { item["context_window"] = contextWindow }
            if let maxOutputTokens = model.maxOutputTokens { item["max_output_tokens"] = maxOutputTokens }
            if let reasoning = model.reasoning { item["reasoning"] = reasoning }
            return item
        }
        let body: [String: Any] = ["object": "list", "data": items]
        return try JSONSerialization.data(withJSONObject: body, options: [.sortedKeys])
    }
}

struct DsgoConfigFile: Decodable {
    struct Listen: Decodable {
        var host: String?
        var port: UInt16?
    }

    struct Upstreams: Decodable {
        var ds4: String?
        var llama: String?
    }

    var listen: Listen?
    var upstreams: Upstreams?
    var models: [RegisteredModel]?

    static func load(path: String) throws -> DsgoConfigFile {
        let url = URL(fileURLWithPath: path).standardizedFileURL
        let data = try Data(contentsOf: url)
        let decoder = JSONDecoder()
        return try decoder.decode(DsgoConfigFile.self, from: data)
    }
}
