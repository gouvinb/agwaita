import {CommandDef, RequestHandler} from "../RequestHandlerParser"
import AstalNotifd from "gi://AstalNotifd"

export class NotificationsRequestHandler implements RequestHandler {
    response: (response: string) => void
    parentCommand: string

    notifd = AstalNotifd.get_default()


    constructor(response: (response: string) => void, parentCommand: string) {
        this.response = response;
        this.parentCommand = parentCommand;
    }

    get cmds(): CommandDef[] {
        return [
            {name: "toggle", args: [], parentCommand: this.parentCommand},
            {name: "show", args: [], parentCommand: this.parentCommand},
            {name: "hide", args: [], parentCommand: this.parentCommand},
            {name: "visible", args: [], parentCommand: this.parentCommand},
        ];
    }

    parse(cmd: string, arg: string | null, ...rest: string[]) {
        if (cmd == "-h" || cmd == "--help" || cmd == "help" || cmd == "") {
            this.help()
        } else if (cmd == "count") {
            if (arg != null) {
                throw `Argument is missing for ${cmd}`
            }

            return this.response(this.notifd.get_notifications().length.toString())
        } else if (cmd == "dismiss-last") {
            if (arg != null) {
                throw `Argument is missing for ${cmd}`
            }

            if (this.notifd.get_notifications().length == 0) {
                throw "No notification to close"
            }

            this.notifd.get_notifications()[0]!.dismiss()
        } else if (cmd == "dnd") {
            if (arg != null) {
                throw `Argument is missing for ${cmd}`
            }

            this.notifd.dontDisturb = !this.notifd.dontDisturb
            return this.response(this.notifd.dontDisturb.toString() ? "on" : "off")
        } else {
            throw `Unknown command: ${cmd} ${arg} ${rest}`
        }
    }

    help(): void {
        this.response(`
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
