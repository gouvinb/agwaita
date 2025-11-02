import {exec, execAsync} from "ags/process"
import GLib from "gi://GLib"

export async function shellAsync(shell: string, shellArgs: string[] = ["-c"], strings: TemplateStringsArray | string, ...values: unknown[]) {
    const cmd = typeof strings === "string" ? strings : strings
        .flatMap((str, i) => str + `${values[i] ?? ""}`)
        .join("")

    return execAsync([shell, ...shellArgs, cmd]).catch(err => {
        console.error(cmd, err)
        return ""
    })
}

export function shell(shell: string, shellArgs: string[] = ["-c"], strings: TemplateStringsArray | string, ...values: unknown[]) {
    const cmd = typeof strings === "string" ? strings : strings
        .flatMap((str, i) => str + `${values[i] ?? ""}`)
        .join("")

    return exec([shell, ...shellArgs, cmd])
}

export async function shAsync(strings: TemplateStringsArray | string, ...values: unknown[]) {
    return shellAsync("sh", ["-c"], strings, ...values)
}

export function sh(strings: TemplateStringsArray | string, ...values: unknown[]) {
    return shell("sh", ["-c"], strings, ...values)
}

export async function bashAsync(strings: TemplateStringsArray | string, ...values: unknown[]) {
    return shellAsync("bash", ["-c"], strings, ...values)
}

export function bash(strings: TemplateStringsArray | string, ...values: unknown[]) {
    return shell("bash", ["-c"], strings, ...values)
}

export async function zshAsync(strings: TemplateStringsArray | string, ...values: unknown[]) {
    return shellAsync("zsh", ["-c"], strings, ...values)
}

export function zsh(strings: TemplateStringsArray | string, ...values: unknown[]) {
    return shell("zsh", ["-c"], strings, ...values)
}

export async function nushellAsync(strings: TemplateStringsArray | string, ...values: unknown[]) {
    return shellAsync("nu", ["-c"], strings, ...values)
}

export function nushell(strings: TemplateStringsArray | string, ...values: unknown[]) {
    return shell("nu", ["-c"], strings, ...values)
}

abstract class Lib {
    protected abstract lib: string

    public execAsync(cmd: string) {
        return execAsync(`${this.pathPrefix()}/${this.lib}/${cmd}`)
    }

    public exec(cmd: string) {
        return exec(`${this.pathPrefix()}/${this.lib}/${cmd}`)
    }

    public pathOf(file: string) {
        return `${this.pathPrefix()}/${this.lib}/${file}`
    }

    protected pathPrefix() {
        return GLib.getenv("XDG_LIB_HOME")
    }
}

class DesktopScriptsLib extends Lib {
    protected lib: string = "desktop-scripts"
}

export default new DesktopScriptsLib
