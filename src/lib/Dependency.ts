import {exec} from "ags/process"
import GLib from "gi://GLib"

/**
 * @returns true if all the `bins` are found
 */
export function dependencies(...bins: string[]) {
    const missing = bins.filter(bin => {
        return !exec(`which ${bin}`)
    })

    if (missing.length > 0) {
        printerr("missing dependencies:", missing.join(", "))
    }

    return missing.length === 0
}

/**
 * @returns true if all the `files` are found
 */
export function customDependencies(fileTest: GLib.FileTest = GLib.FileTest.EXISTS, ...files: string[]) {
    if (files.length === 0) {
        printerr("missing dependencies: no files provided")
        return false
    }

    const missing = files.filter(file => {
        return !GLib.file_test(file, fileTest)
    })

    if (missing.length > 0) {
        printerr("missing dependencies:", missing.join(", "))
    }

    return missing.length === 0
}

/**
 * @returns true if all the `bins` are found
 */
export function customExecutableDependencies(...bins: string[]) {
    return customDependencies(GLib.FileTest.IS_EXECUTABLE, ...bins)
}

/**
 * @returns true if all the `dirs` are found
 */
export function customDirDependencies(...dirs: string[]) {
    return customDependencies(GLib.FileTest.IS_DIR, ...dirs)
}
