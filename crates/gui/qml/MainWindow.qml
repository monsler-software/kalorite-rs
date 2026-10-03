import QtQuick;
import QtQuick.Controls;
import QtQuick.Layouts;
import gui;

ApplicationWindow {
    visible: true
    width: 350
    height: 600
    title: 'Kalorite'

    property int position: 0
    property int volume: 0

    Timer {
        interval: 30
        running: true
        repeat: true
        onTriggered: MainWindow.poll_audio_events()
    }

    Connections {
        target: MainWindow

        function onVolumeChanged(vol) {
            volume = vol
            volumeBox.value = volume 
        }
    }

    ColumnLayout {
        Button {
            text: 'Play'
            onClicked: MainWindow.play()
        }

        Button {
            text: 'Stop'
            onClicked: MainWindow.stop()
        }

        Button {
            text: 'Pause'
            onClicked: MainWindow.pause()
        }

        Button {
            text: 'Pause'
            onClicked: MainWindow.get_volume()
        }

        Text {
            text: "Volume: " + volume + "%"
        }
        
        SpinBox {
            id: volumeBox
            value: 100.0
            onValueChanged: {
                MainWindow.set_volume(volumeBox.value)
                MainWindow.volumeChanged(volumeBox.value)
            }

        }
    }
}
