import {CommandDef, RequestHandler} from "../RequestHandlerParser"
import app from "ags/gtk4/app";

export class BluetoothRequestHandler implements RequestHandler {
    response: (response: string) => void
    parentCommand: string

    constructor(response: (response: string) => void, parentCommand: string) {
        this.response = response;
        this.parentCommand = parentCommand;
    }

    get cmds(): CommandDef[] {
        return [
            {name: "open", args: [], parentCommand: this.parentCommand},
            {name: "close", args: [], parentCommand: this.parentCommand},
        ];
    }

    parse(cmd: string, arg: string | null, ...rest: string[]) {
        if (cmd == "-h" || cmd == "--help" || cmd == "help" || cmd == "") {
            this.help()
        } else if (cmd == "open") {
            if (arg != null) {
                throw `Argument is missing for ${cmd}`
            }

            const bluetoothWindow = app.get_window("bluetoothctl.gui")
            if (!bluetoothWindow) {
                throw "bluetoothctl.gui window not found"
            }
            bluetoothWindow.show()

            return this.response("")
        } else if (cmd == "close") {
            if (arg != null) {
                throw `Argument is missing for ${cmd}`
            }

            const bluetoothWindow = app.get_window("bluetoothctl.gui")
            if (!bluetoothWindow) {
                throw "bluetoothctl.gui window not found"
            }
            bluetoothWindow.close()
            return this.response("")
        } else if (cmd != undefined) {
            throw `Unknown command: ${this.parentCommand} ${cmd}${arg ? ` ${arg}` : ``}${rest ? ` ${rest}` : ``}`
        } else {
            throw `Argument is missing for ${this.parentCommand}`
        }
    }

    help(msg?: string): void {
        this.response(
            (msg ? `${msg}\n` : ``) +
            `
                |Usage:
                |  > ags request ${this.parentCommand} <action>
                |
                |Subcommands:
                |${this.cmds.map((cmd) => `  ags request ${this.parentCommand} ${cmd.name}`).join("\n|")}
                |
                |Flags:
                |  -h, --help: Display the help message for this command
                |
                |Parameters:
                |  action <string>: ${this.cmds.map((cmd) => cmd.name).join(", ")}
                |
                |Input/output types:
                |string | nothing
            `
                .split("\n")
                .map(line => line.trimStart().replace("|", ""))
                .join("\n")
        )
    }
}
