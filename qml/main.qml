import QtQuick
import QtQuick.Controls
import QtQuick.Window

import dev.zedsoftworks.amplitude

ApplicationWindow {
    id: root
    height: 1280
    width: 720
    visible: true
    color: palette.window

    PwCore {
        id: pwCore

        Component.onCompleted: {
            if (!pwCore.initialize()) {
                console.error("Failed to initialize pipewire")
                errorDialog.open()
            }
        }

        Component.onDestruction: {
            pwCore.cleanup()
        }
    }

    Column {
        PwCore {
        }

        anchors.fill: parent
        anchors.margins: 10
        spacing: 10

        Label {
            text: "Hello, World!"
            color: palette.text
        }
    }

}
