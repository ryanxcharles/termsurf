import RoasttyKit
import Testing
@testable import Roastty

struct SurfaceKeyTextTests {
    @Test func keyEventWithCValuePreservesTextAndComposing() {
        let event = Roastty.Input.KeyEvent(
            key: .a,
            text: "é",
            composing: true,
            mods: [.shift],
            consumedMods: [.shift],
            unshiftedCodepoint: UnicodeScalar("e").value
        )

        event.withCValue { cEvent in
            #expect(cEvent.action == ROASTTY_ACTION_PRESS)
            #expect(cEvent.keycode == 0)
            #expect(cEvent.composing)
            #expect(cEvent.mods == ROASTTY_MODS_SHIFT)
            #expect(cEvent.consumed_mods == ROASTTY_MODS_SHIFT)
            #expect(cEvent.unshifted_codepoint == UnicodeScalar("e").value)
            #expect(cEvent.text != nil)
            #expect(String(cString: cEvent.text!) == "é")
        }
    }

    @Test func keyEventWithCValuePreservesNilText() {
        let event = Roastty.Input.KeyEvent(
            key: .escape,
            action: .release,
            text: nil,
            composing: false
        )

        event.withCValue { cEvent in
            #expect(cEvent.action == ROASTTY_ACTION_RELEASE)
            #expect(cEvent.keycode == 0x35)
            #expect(!cEvent.composing)
            #expect(cEvent.text == nil)
        }
    }
}
