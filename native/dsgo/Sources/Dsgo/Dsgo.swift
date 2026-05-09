import Darwin
import Dispatch
import Foundation

struct CliArgs {
    var configPath: String?
    var host = "127.0.0.1"
    var port: UInt16 = 38442
    var llamaUpstream = "http://127.0.0.1:18081"
    var ds4Upstream = "http://127.0.0.1:8000"
    var ds4Models = "deepseek-v4-flash"
    var maxStreamBytes = 200 * 1024 * 1024

    var spawnLlama = false
    var llamaBin: String?
    var llamaModelPath: String?
    var llamaPort: UInt16 = 18081
    var llamaExtra: [String] = []

    var spawnDs4 = false
    var ds4Bin: String?
    var ds4ModelPath: String?
    var ds4Port: UInt16 = 8000
    var ds4Extra: [String] = []
}

enum CliParse {
    static func argsWithUsage(_ argv: [String]) throws -> CliArgs {
        guard argv.count >= 1 else { throw NSError(domain: "dsgo", code: 1) }
        var a = CliArgs()
        if let v = ProcessInfo.processInfo.environment["DSGO_MAX_STREAM_BYTES"], let n = Int(v), n > 0 {
            a.maxStreamBytes = n
        }

        var i = 1
        outer: while i < argv.count {
            let flag = argv[i]
            func next() throws -> String {
                i += 1
                guard i < argv.count else { throw NSError(domain: "dsgo", code: 1, userInfo: [NSLocalizedDescriptionKey: "missing value after \(flag)"]) }
                return argv[i]
            }
            switch flag {
            case "--help", "-h":
                printUsage()
                exit(0)
            case "--host":
                a.host = try next()
            case "--config":
                a.configPath = try next()
            case "--port":
                guard let p = UInt16(try next()), p > 0 else {
                    throw NSError(domain: "dsgo", code: 1, userInfo: [NSLocalizedDescriptionKey: "bad port"])
                }
                a.port = p
            case "--llama-upstream":
                a.llamaUpstream = try next()
            case "--ds4-upstream":
                a.ds4Upstream = try next()
            case "--ds4-models":
                a.ds4Models = try next()
            case "--max-stream-bytes":
                guard let n = Int(try next()), n > 0 else { throw NSError(domain: "dsgo", code: 1, userInfo: [NSLocalizedDescriptionKey: "bad max-stream-bytes"]) }
                a.maxStreamBytes = n
            case "--spawn-llama":
                a.spawnLlama = true
            case "--llama-bin":
                a.llamaBin = try next()
            case "--llama-model":
                a.llamaModelPath = try next()
            case "--llama-port":
                guard let p = UInt16(try next()), p > 0 else { throw NSError(domain: "dsgo", code: 1, userInfo: [NSLocalizedDescriptionKey: "bad llama-port"]) }
                a.llamaPort = p
            case "--llama-arg":
                a.llamaExtra.append(try next())
            case "--spawn-ds4":
                a.spawnDs4 = true
            case "--ds4-bin":
                a.ds4Bin = try next()
            case "--ds4-model":
                a.ds4ModelPath = try next()
            case "--ds4-port":
                guard let p = UInt16(try next()), p > 0 else { throw NSError(domain: "dsgo", code: 1, userInfo: [NSLocalizedDescriptionKey: "bad ds4-port"]) }
                a.ds4Port = p
            case "--ds4-arg":
                a.ds4Extra.append(try next())
            case "--":
                i += 1
                while i < argv.count {
                    a.llamaExtra.append(argv[i])
                    i += 1
                }
                break outer
            default:
                throw NSError(domain: "dsgo", code: 1, userInfo: [NSLocalizedDescriptionKey: "unknown flag \(flag)"])
            }
            i += 1
        }
        return a
    }

    static func printUsage() {
        let text = """
        dsgo — Anthropic Messages API gateway (routes by model to llama-server vs ds4-server).

        Usage:
          dsgo [options]

        Gateway:
          --config PATH                optional JSON config (listen/upstreams/models)
          --host ADDR                  default 127.0.0.1
          --port PORT                  default 38442
          --llama-upstream URL         default http://127.0.0.1:18081
          --ds4-upstream URL           default http://127.0.0.1:8000
          --ds4-models CSV             default deepseek-v4-flash
          --max-stream-bytes N         buffer cap for streaming responses (default 200MiB; env DSGO_MAX_STREAM_BYTES)

        Optional helpers (spawn local backends before listening):
          --spawn-llama
          --llama-bin PATH             default: Bundles/llama-server next to dsgo, else PATH lookup
          --llama-model PATH           GGUF path (required if --spawn-llama)
          --llama-port PORT            default 18081 (updates effective --llama-upstream unless overridden)
          --llama-arg ARG              extra argument forwarded to llama-server (repeatable)

          --spawn-ds4
          --ds4-bin PATH               default: ds4-server next to dsgo, else PATH lookup
          --ds4-model PATH             GGUF path (passed through as `ds4-server -m PATH`)
          --ds4-port PORT              default 8000 (updates effective --ds4-upstream unless overridden)
          --ds4-arg ARG                extra argument forwarded to ds4-server (repeatable)

          -- EXTRA_ARGS                 extra arguments forwarded to llama-server only

        Distribution note:
          Place `llama-server` and/or `ds4-server` beside the `dsgo` executable to bundle backends without PATH.

        """
        fputs(text, stderr)
    }
}

enum BundledBinary {
    static func dir() -> URL {
        URL(fileURLWithPath: CommandLine.arguments[0]).standardizedFileURL.deletingLastPathComponent()
    }

    static func sibling(_ name: String) -> URL {
        dir().appendingPathComponent(name, isDirectory: false)
    }

    static func which(_ name: String) -> URL? {
        let p = Process()
        p.executableURL = URL(fileURLWithPath: "/usr/bin/which")
        p.arguments = [name]
        let pipe = Pipe()
        p.standardOutput = pipe
        p.standardError = Pipe()
        do {
            try p.run()
            p.waitUntilExit()
            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            guard let s = String(data: data, encoding: .utf8)?.trimmingCharacters(in: .newlines), !s.isEmpty else { return nil }
            return URL(fileURLWithPath: s)
        } catch {
            return nil
        }
    }
}

extension CliArgs {
    mutating func apply(config: DsgoConfigFile) {
        if let host = config.listen?.host { self.host = host }
        if let port = config.listen?.port { self.port = port }
        if let llama = config.upstreams?.llama { self.llamaUpstream = llama }
        if let ds4 = config.upstreams?.ds4 { self.ds4Upstream = ds4 }
    }
}

final class ChildProcesses {
    private var processes: [Process] = []
    private let lock = NSLock()

    func add(_ p: Process) {
        lock.lock()
        processes.append(p)
        lock.unlock()
    }

    func terminateAll() {
        lock.lock()
        let ps = processes
        processes.removeAll()
        lock.unlock()
        for p in ps where p.isRunning {
            p.terminate()
        }
    }
}

final class LockedValue<T>: @unchecked Sendable {
    private let lock = NSLock()
    private var value: T

    init(_ value: T) {
        self.value = value
    }

    func set(_ newValue: T) {
        lock.lock()
        value = newValue
        lock.unlock()
    }

    func get() -> T {
        lock.lock()
        let current = value
        lock.unlock()
        return current
    }
}

enum Spawn {
    static func llama(args: CliArgs, children: ChildProcesses) throws {
        guard let modelPath = args.llamaModelPath else {
            throw NSError(domain: "dsgo", code: 1, userInfo: [NSLocalizedDescriptionKey: "--spawn-llama requires --llama-model"]) }
        let binURL: URL = if let b = args.llamaBin {
            URL(fileURLWithPath: b)
        } else if FileManager.default.isExecutableFile(atPath: BundledBinary.sibling("llama-server").path) {
            BundledBinary.sibling("llama-server")
        } else if let w = BundledBinary.which("llama-server") {
            w
        } else {
            throw NSError(domain: "dsgo", code: 1, userInfo: [NSLocalizedDescriptionKey: "llama-server not found; use --llama-bin or place llama-server beside dsgo"]) }

        let modelURL = URL(fileURLWithPath: modelPath)
        var cmd: [String] = [
            "--host", "127.0.0.1",
            "--port", "\(args.llamaPort)",
            "-m", modelURL.path,
            "-ngl", "auto",
            "--no-webui",
        ]
        cmd.append(contentsOf: args.llamaExtra)

        let p = Process()
        p.executableURL = binURL
        p.arguments = cmd
        try p.run()
        children.add(p)
        fputs("dsgo: spawned llama-server pid=\(p.processIdentifier) port=\(args.llamaPort)\n", stderr)
    }

    static func ds4(args: CliArgs, children: ChildProcesses) throws {
        let binURL: URL = if let b = args.ds4Bin {
            URL(fileURLWithPath: b)
        } else if FileManager.default.isExecutableFile(atPath: BundledBinary.sibling("ds4-server").path) {
            BundledBinary.sibling("ds4-server")
        } else if let w = BundledBinary.which("ds4-server") {
            w
        } else {
            throw NSError(domain: "dsgo", code: 1, userInfo: [NSLocalizedDescriptionKey: "ds4-server not found; use --ds4-bin or place ds4-server beside dsgo"]) }

        var cmd: [String] = ["--host", "127.0.0.1", "--port", "\(args.ds4Port)"]
        if let mp = args.ds4ModelPath {
            cmd.insert(contentsOf: ["-m", mp], at: 0)
        }
        cmd.append(contentsOf: args.ds4Extra)

        let p = Process()
        p.executableURL = binURL
        p.arguments = cmd
        try p.run()
        children.add(p)
        fputs("dsgo: spawned ds4-server pid=\(p.processIdentifier) bind=127.0.0.1:\(args.ds4Port)\n", stderr)
    }
}

enum BackendReadiness {
    static func wait(baseURL: String, label: String, timeoutSeconds: TimeInterval = 30) -> Bool {
        guard let url = URL(string: baseURL.trimmingCharacters(in: CharacterSet(charactersIn: "/")) + "/v1/models") else {
            return false
        }
        let deadline = Date().addingTimeInterval(timeoutSeconds)
        while Date() < deadline {
            var req = URLRequest(url: url)
            req.httpMethod = "GET"
            req.timeoutInterval = 2

            let sema = DispatchSemaphore(value: 0)
            let ok = LockedValue(false)
            let task = ProxySession.shared.dataTask(with: req) { _, resp, _ in
                if let http = resp as? HTTPURLResponse, http.statusCode < 500 {
                    ok.set(true)
                }
                sema.signal()
            }
            task.resume()
            _ = sema.wait(timeout: .now() + 2)
            if ok.get() {
                fputs("dsgo: \(label) ready at \(baseURL)\n", stderr)
                return true
            }
            Thread.sleep(forTimeInterval: 0.25)
        }
        fputs("dsgo: warning: \(label) did not become ready within \(Int(timeoutSeconds))s; continuing gateway startup\n", stderr)
        return false
    }
}

@main
enum Entry {
    static func main() {
        var cli: CliArgs
        do {
            cli = try CliParse.argsWithUsage(CommandLine.arguments)
            if let configPath = cli.configPath {
                let file = try DsgoConfigFile.load(path: configPath)
                cli.apply(config: file)
            }
        } catch {
            fputs("dsgo: \(error.localizedDescription)\n", stderr)
            CliParse.printUsage()
            exit(1)
        }

        var llamaURL = cli.llamaUpstream
        var ds4URL = cli.ds4Upstream
        if cli.spawnLlama {
            llamaURL = "http://127.0.0.1:\(cli.llamaPort)"
        }
        if cli.spawnDs4 {
            ds4URL = "http://127.0.0.1:\(cli.ds4Port)"
        }

        let ds4Models = Set(cli.ds4Models.split(separator: ",").map { String($0.trimmingCharacters(in: .whitespaces)) }.filter { !$0.isEmpty })
        let registry: ModelRegistry
        do {
            if let configPath = cli.configPath {
                let file = try DsgoConfigFile.load(path: configPath)
                registry = ModelRegistry(models: file.models ?? ModelRegistry.defaults(ds4Models: ds4Models).models)
            } else {
                registry = .defaults(ds4Models: ds4Models)
            }
        } catch {
            fputs("dsgo: failed to load model registry: \(error.localizedDescription)\n", stderr)
            exit(1)
        }
        let cfg = GatewayConfig(llamaUpstream: llamaURL, ds4Upstream: ds4URL, models: registry)

        let children = ChildProcesses()
        signal(SIGINT, SIG_IGN)
        let sig = DispatchSource.makeSignalSource(signal: SIGINT, queue: .main)
        sig.setEventHandler {
            fputs("\ndsgo: shutting down…\n", stderr)
            children.terminateAll()
            exit(130)
        }
        sig.resume()

        do {
            if cli.spawnLlama { try Spawn.llama(args: cli, children: children) }
            if cli.spawnDs4 { try Spawn.ds4(args: cli, children: children) }
        } catch {
            fputs("dsgo: \(error.localizedDescription)\n", stderr)
            children.terminateAll()
            exit(1)
        }
        if cli.spawnLlama { _ = BackendReadiness.wait(baseURL: llamaURL, label: "llama-server") }
        if cli.spawnDs4 { _ = BackendReadiness.wait(baseURL: ds4URL, label: "ds4-server") }

        let server = GatewayHTTPServer(config: cfg, host: cli.host, port: cli.port, maxStreamBytes: cli.maxStreamBytes)
        do {
            try server.start()
        } catch {
            fputs("dsgo: failed to listen on \(cli.host):\(cli.port): \(error.localizedDescription)\n", stderr)
            children.terminateAll()
            exit(1)
        }

        if cli.host != "0.0.0.0" {
            fputs("dsgo: note: Network.framework listener currently accepts on all interfaces; keep this process behind local firewall rules until POSIX bind is added.\n", stderr)
        }
        fputs("dsgo: gateway http://\(cli.host):\(cli.port)  (llama \(cfg.llamaUpstream) | ds4 \(cfg.ds4Upstream))\n", stderr)

        dispatchMain()
    }
}
