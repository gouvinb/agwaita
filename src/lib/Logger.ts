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
} as const; // 'as const' garantit que les valeurs sont des littéraux exacts ("C", "E", etc.)

// Définition des types pour une utilisation stricte
type LogLevelKey = keyof typeof LogLevel; // 'CRITICAL' | 'ERROR' | 'WARNING' | 'INFO' | 'DEBUG'
type LogLevelValue = typeof LogLevel[LogLevelKey]; // 'C' | 'E' | 'W' | 'I' | 'D'


export const Log = new class {
    private printLog(level: LogLevelValue, tag: string, msg: string, err?: Error | unknown | null | undefined) {
        const date = new Date()

        const log = `${date.toLocaleDateString()}|${level}|${tag.substring(0, 25).padEnd(25,"_")}|${msg}${err ? `: ${err}` : ""}`

        if (level === "C" || level === "E" || level === "W") {
            printerr(log)
        } else if (level === "I" || level === "D") {
            print(log)
        }
    }

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
}
