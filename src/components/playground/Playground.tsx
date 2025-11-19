import Adw from "gi://Adw"
import app from "ags/gtk4/app"

export default function Playground() {
    return (
        <Adw.Window
            application={app}
            name="playground"
            title="Playground"
            visible={false}
        >
        </Adw.Window>
    )
}
