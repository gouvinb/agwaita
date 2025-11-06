/**
 * @type {Readonly<Record<LogLevelKey, string>>}
 * L'objet 'LogLevel' mappe le niveau (clé) à son code court (valeur).
 */
const LogLevel = {
    CRITICAL: "C",
    ERROR: "E",
    WARNING: "W",
    INFO: "I",
    DEBUG: "D",
} as const;

type LogLevelKey = keyof typeof LogLevel;
type LogLevelValue = typeof LogLevel[LogLevelKey];

export const Log = new class {
    c(tag: string, msg: string, err?: Error | unknown | null | undefined) {
        this.printLog(LogLevel.CRITICAL, tag, msg, err)
    }

    e(tag: string, msg: string, err?: Error | unknown | null | undefined) {
        this.printLog(LogLevel.ERROR, tag, msg, err)
    }

    w(tag: string, msg: string, err?: Error | unknown | null | undefined) {
        this.printLog(LogLevel.WARNING, tag, msg, err)
    }

    i(tag: string, msg: string) {
        this.printLog(LogLevel.INFO, tag, msg)
    }

    d(tag: string, msg: string) {
        this.printLog(LogLevel.DEBUG, tag, msg)
    }

    private printLog(level: LogLevelValue, tag: string, msg: string, err?: Error | unknown | null | undefined) {
        const date = new Date()

        const log = `${date.toLocaleDateString()}|${level}|${tag.substring(0, 25).padEnd(25, "_")}|${msg}${err ? `: ${err}` : ""}`

        if (level === "C" || level === "E" || level === "W") {
            printerr(log)
        } else if (level === "I" || level === "D") {
            print(log)
        }
    }
}
