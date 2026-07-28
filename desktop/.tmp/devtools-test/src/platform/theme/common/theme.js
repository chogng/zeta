/** Color scheme used by the operating system and color themes. */
export var ColorScheme;
(function (ColorScheme) {
    ColorScheme["Dark"] = "dark";
    ColorScheme["Light"] = "light";
    ColorScheme["HighContrastDark"] = "high-contrast-dark";
    ColorScheme["HighContrastLight"] = "high-contrast-light";
})(ColorScheme || (ColorScheme = {}));
export function isDarkColorScheme(scheme) {
    return scheme === ColorScheme.Dark ||
        scheme === ColorScheme.HighContrastDark;
}
