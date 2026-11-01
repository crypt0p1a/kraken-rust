import Foundation

#if canImport(Glibc)
import Glibc
#elseif canImport(Darwin)
import Darwin
#endif

/// A local HTTP server answering like Kraken: a JSON body per path, on 127.0.0.1. On plain POSIX
/// sockets, so that the same tests run on macOS, on the iOS simulator and on Linux
final class MockKraken: @unchecked Sendable {
    struct Answer {
        let status: Int
        let body: String
    }

    private let listener: Int32
    private let lock = NSLock()
    private var answers: [String: Answer] = [:]
    private var receivedRequests: [String] = []
    let port: UInt16

    var host: String { "http://127.0.0.1:\(port)" }

    init() throws {
        #if canImport(Glibc)
        let stream = Int32(SOCK_STREAM.rawValue)
        #else
        let stream = SOCK_STREAM
        #endif
        let fd = socket(AF_INET, stream, 0)
        guard fd >= 0 else { throw MockError.socket("socket") }

        var yes: Int32 = 1
        setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &yes, socklen_t(MemoryLayout<Int32>.size))

        var address = sockaddr_in()
        #if canImport(Darwin)
        address.sin_len = UInt8(MemoryLayout<sockaddr_in>.size)
        #endif
        address.sin_family = sa_family_t(AF_INET)
        address.sin_port = 0
        address.sin_addr.s_addr = inet_addr("127.0.0.1")
        let bound = withUnsafePointer(to: &address) {
            $0.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                bind(fd, $0, socklen_t(MemoryLayout<sockaddr_in>.size))
            }
        }
        guard bound == 0, listen(fd, 16) == 0 else { throw MockError.socket("bind / listen") }

        var length = socklen_t(MemoryLayout<sockaddr_in>.size)
        _ = withUnsafeMutablePointer(to: &address) {
            $0.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                getsockname(fd, $0, &length)
            }
        }
        listener = fd
        port = UInt16(bigEndian: address.sin_port)

        let thread = Thread { [self] in serve() }
        thread.start()
    }

    deinit {
        close(listener)
    }

    /// Answers `body` to the calls of `path`, e.g. "/0/public/Ticker"
    func answer(_ path: String, status: Int = 200, body: String) {
        lock.lock()
        answers[path] = Answer(status: status, body: body)
        lock.unlock()
    }

    /// The requests received, headers and body
    var received: [String] {
        lock.lock()
        defer { lock.unlock() }
        return receivedRequests
    }

    /// The request lines received, e.g. "POST /0/private/BalanceEx HTTP/1.1"
    var requests: [String] {
        received.map { $0.components(separatedBy: "\r\n").first ?? "" }
    }

    private func serve() {
        while true {
            let client = accept(listener, nil, nil)
            if client < 0 { return }
            handle(client)
            close(client)
        }
    }

    private func handle(_ client: Int32) {
        var data = Data()
        var buffer = [UInt8](repeating: 0, count: 4096)
        // the headers, then the body announced by Content-Length
        while true {
            let count = recv(client, &buffer, buffer.count, 0)
            if count <= 0 { return }
            data.append(buffer, count: count)
            guard let text = String(data: data, encoding: .utf8),
                  let end = text.range(of: "\r\n\r\n") else { continue }
            let headers = text[..<end.lowerBound].lowercased()
            let length = headers.split(separator: "\r\n")
                .first { $0.hasPrefix("content-length:") }
                .flatMap { Int($0.dropFirst("content-length:".count).trimmingCharacters(in: .whitespaces)) } ?? 0
            if text[end.upperBound...].utf8.count >= length { break }
        }

        let text = String(decoding: data, as: UTF8.self)
        let line = text.components(separatedBy: "\r\n").first ?? ""
        let path = line.split(separator: " ").dropFirst().first
            .map { String($0.split(separator: "?").first ?? "") } ?? ""

        lock.lock()
        receivedRequests.append(text)
        let answer = answers[path] ?? Answer(status: 404, body: "not mocked: \(path)")
        lock.unlock()

        let body = Array(answer.body.utf8)
        let head = "HTTP/1.1 \(answer.status) Mock\r\nContent-Type: application/json\r\n"
            + "Content-Length: \(body.count)\r\nConnection: close\r\n\r\n"
        let response = Array(head.utf8) + body
        var sent = 0
        while sent < response.count {
            let count = response[sent...].withUnsafeBytes { send(client, $0.baseAddress, $0.count, 0) }
            if count <= 0 { return }
            sent += count
        }
    }
}

enum MockError: Error {
    case socket(String)
}
