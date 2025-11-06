export interface RequestHandler {
    cmds: CommandDef[]

    parse(cmd: string, arg: string | null | undefined, ...rest: string[]): void

    help(msg?: string): void
}


export type CommandDef = {
    parentCommand?: string | null
    name: string
    description?: string | null
    args: string[]
    handler?: RequestHandler | null
}
