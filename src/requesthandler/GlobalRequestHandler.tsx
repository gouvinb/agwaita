import {CommandDef, RequestHandler} from "./RequestHandlerParser"
import {BarRequestHandler} from "./impl/BarRequestHandler"
import {NotificationsRequestHandler} from "./impl/NotificationsRequestHandler"

export class GlobalRequestHandler implements RequestHandler {
    response: (response: string) => void

    constructor(response: (response: string) => void) {
        this.response = response
    }

    get cmds(): CommandDef[] {
        return [
            {
                name: "bar",
                handler: new BarRequestHandler(this.response, "bar"),
                args: [],
            },
            {
                name: "notifications",
                handler: new NotificationsRequestHandler(this.response, "notifications"),
                args: [],
            },
        ]
    }

    parse(cmd: string, arg: string | null | undefined, ...rest: string[]) {
        if (cmd == "-h" || cmd == "--help" || cmd == "help" || cmd == "") {
            this.help()
        } else {
            const handler = this.cmds.find((command) => command.name == cmd)?.handler
            if (handler != undefined) {
                handler.parse(arg!, rest[0], ...rest.slice(1))
            } else {
                throw `Unknown command: ${cmd}${arg ? ` ${arg}` : ``}${rest ? ` ${rest}` : ``}`
            }
        }
    }

    help(msg?: string): void {
        this.response(
            (msg ? `${msg}\n` : ``) +
            `
                |Usage:
                |  > ags request <action>
                |
                |Subcommands:
                |${this.cmds.map((cmd) => `  ags request ${cmd.name}`).join("\n|")}
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
