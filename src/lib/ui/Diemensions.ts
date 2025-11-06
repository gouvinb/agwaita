export const Dimensions = new class {
    // /////////////////////////////////////////////////////////////////////////
    // Default spacing.
    // /////////////////////////////////////////////////////////////////////////

    /**
     * Value:  0
     */
    noSpacing = 0
    /**
     * Value:  2
     */
    smallestSpacing = 2
    /**
     * Value:  4
     */
    smallSpacing = 4
    /**
     * Value:  6
     */
    semiSmallSpacing = 6
    /**
     * Value:  8
     */
    normalSpacing = 8
    /**
     * Value: 12
     */
    semiBigSpacing = 12
    /**
     * Value: 16
     */
    bigSpacing = 16
    /**
     * Value: 24
     */
    semiBiggerSpacing = 24
    /**
     * Value: 32
     */
    biggerSpacing = 32
    /**
     * Value: 64
     */
    biggestSpacing = 64

    // /////////////////////////////////////////////////////////////////////////
    // Container specific sizes
    // /////////////////////////////////////////////////////////////////////////

    quickSettingsWidth = 320
    quickSettingsColumnWidth = 100

    notificationWidth = 320
    notificationCenterWidth = this.notificationWidth * 2
    notificationCenterHeight = 640
}
