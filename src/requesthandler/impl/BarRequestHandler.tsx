import {CommandDef, RequestHandler} from "../RequestHandlerParser"
import app from "ags/gtk4/app"
import {Gtk} from "ags/gtk4"

export class BarRequestHandler implements RequestHandler {
    response: (response: string) => void
    parentCommand: string

    bar: Gtk.Window

    constructor(response: (response: string) => void, parentCommand: string) {
        this.response = response
        this.parentCommand = parentCommand

        const bar = app.get_window("bar")
        if (!bar) {
            throw "bar window not found"
        }
        this.bar = bar;
    }


    get cmds(): CommandDef[] {
        return [
            {name: "toggle", args: [], parentCommand: this.parentCommand},
            {name: "show", args: [], parentCommand: this.parentCommand},
            {name: "hide", args: [], parentCommand: this.parentCommand},
            {name: "visible", args: [], parentCommand: this.parentCommand},
        ]
    }

    parse(cmd: string, arg: string | null, ...rest: string[]) {
        if (cmd == "-h" || cmd == "--help" || cmd == "help" || cmd == "") {
            this.help()
        } else if (cmd == "toggle") {
            if (arg != null) {
                throw `Unknown argument: ${arg} for ${cmd}`
            }

            this.bar.visible = !this.bar.visible
            return this.response(this.bar.visible ? "hide" : "show")
        } else if (cmd == "show") {
            if (arg != null) {
                throw `Unknown argument: ${arg} for ${cmd}`
            }

            this.bar.visible = true
            return this.response("show")
        } else if (cmd == "hide") {
            if (arg != null) {
                throw `Unknown argument: ${arg} for ${cmd}`
            }

            this.bar.visible = false
            return this.response("hide")
        } else if (cmd == "visible") {
            if (arg != null) {
                throw `Unknown argument: ${arg} for ${cmd}`
            }

            return this.response(this.bar.visible.toString())
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
