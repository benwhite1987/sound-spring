import QtQuick
import QtQuick.Controls
import com.benkahn.soundboard

Button {
    id: root

    // secondary | primary | danger
    property string role: "secondary"

    SoundSpringTheme {
        id: appTheme
    }

    padding: 10
    palette.buttonText: appTheme.textPrimary

    background: Rectangle {
        radius: 5
        border.width: root.hovered || root.down ? 1 : 0
        border.color: {
            if (root.role === "primary" && (root.hovered || root.down))
                return appTheme.borderAccent
            return appTheme.border
        }
        color: {
            if (!root.enabled)
                return appTheme.surface

            if (root.role === "danger") {
                if (root.down)
                    return appTheme.danger
                if (root.hovered)
                    return appTheme.dangerHover
                return appTheme.danger
            }

            if (root.role === "primary") {
                if (root.down || root.hovered)
                    return appTheme.surfaceActive
                return "transparent"
            }

            if (root.down || root.hovered)
                return appTheme.surfaceHover
            return "transparent"
        }
    }
}
