import Foundation
import Testing

@Suite(.serialized)
struct ShellIntegrationResourceTests {
    @Test
    func bundledShellIntegrationResourcesAreReachable() throws {
        let resources = try #require(Bundle.main.resourceURL)
        let sentinel = resources.appendingPathComponent("terminfo/78/xterm-roastty", isDirectory: false)
        let terminfoSource = resources.appendingPathComponent("terminfo/roastty.terminfo", isDirectory: false)
        let root = resources.appendingPathComponent("roastty/shell-integration", isDirectory: true)

        #expect(FileManager.default.fileExists(atPath: sentinel.path), "missing \(sentinel.path)")
        #expect(FileManager.default.fileExists(atPath: terminfoSource.path), "missing \(terminfoSource.path)")
        #expect(try Data(contentsOf: sentinel).isEmpty == false)
        #expect(try String(contentsOf: terminfoSource, encoding: .utf8).hasPrefix("xterm-roastty|roastty|Roastty,"))

        let files = [
            "bash/roastty.bash",
            "bash/bash-preexec.sh",
            "zsh/.zshenv",
            "zsh/roastty-integration",
            "fish/vendor_conf.d/roastty-shell-integration.fish",
            "elvish/lib/roastty-integration.elv",
            "nushell/vendor/autoload/roastty.nu",
        ]

        for file in files {
            let url = root.appendingPathComponent(file, isDirectory: false)
            #expect(FileManager.default.fileExists(atPath: url.path), "missing \(url.path)")
        }
    }
}
