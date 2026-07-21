const BASIC_COLORS = ["black", "red", "green", "yellow", "blue", "magenta", "cyan", "white"];

function defaultStyle() {
  return { foreground: null, rgb: null, bright: false, bold: false, dim: false };
}

function styleClass(style) {
  return [
    style.foreground && `ansi-${style.bright ? "bright-" : ""}${style.foreground}`,
    style.rgb && "ansi-extended",
    style.bold && "ansi-bold",
    style.dim && "ansi-dim",
  ]
    .filter(Boolean)
    .join(" ");
}

function relativeLuminance(rgb) {
  const [red, green, blue] = rgb.map((value) => {
    const channel = value / 255;
    return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
  });
  return red * 0.2126 + green * 0.7152 + blue * 0.0722;
}

function contrastRatio(first, second) {
  const lighter = Math.max(relativeLuminance(first), relativeLuminance(second));
  const darker = Math.min(relativeLuminance(first), relativeLuminance(second));
  return (lighter + 0.05) / (darker + 0.05);
}

export function lightThemeColor(rgb) {
  const terminalBackground = [244, 241, 234];
  let candidate = [...rgb];
  while (contrastRatio(candidate, terminalBackground) < 4.5) {
    candidate = candidate.map((value) => Math.floor(value * 0.9));
  }
  return candidate;
}

function xtermColor(index) {
  const palette = [
    [0, 0, 0],
    [205, 49, 49],
    [13, 188, 121],
    [229, 229, 16],
    [36, 114, 200],
    [188, 63, 188],
    [17, 168, 205],
    [229, 229, 229],
    [102, 102, 102],
    [241, 76, 76],
    [35, 209, 139],
    [245, 245, 67],
    [59, 142, 234],
    [214, 112, 214],
    [41, 184, 219],
    [255, 255, 255],
  ];
  if (index < 16) return palette[index];
  if (index < 232) {
    const offset = index - 16;
    const component = (value) => (value === 0 ? 0 : 55 + value * 40);
    return [
      component(Math.floor(offset / 36)),
      component(Math.floor(offset / 6) % 6),
      component(offset % 6),
    ];
  }
  const gray = 8 + (index - 232) * 10;
  return [gray, gray, gray];
}

function validByte(value) {
  return Number.isInteger(value) && value >= 0 && value <= 255;
}

function applySgr(style, parameterText) {
  const parameters = parameterText === "" ? [0] : parameterText.split(";").map(Number);
  for (let index = 0; index < parameters.length; index += 1) {
    const parameter = parameters[index];
    if (parameter === 0) Object.assign(style, defaultStyle());
    else if (parameter === 1) style.bold = true;
    else if (parameter === 2) style.dim = true;
    else if (parameter === 22) {
      style.bold = false;
      style.dim = false;
    } else if (parameter === 39) {
      style.foreground = null;
      style.rgb = null;
      style.bright = false;
    } else if (parameter >= 30 && parameter <= 37) {
      style.foreground = BASIC_COLORS[parameter - 30];
      style.rgb = null;
      style.bright = false;
    } else if (parameter >= 90 && parameter <= 97) {
      style.foreground = BASIC_COLORS[parameter - 90];
      style.rgb = null;
      style.bright = true;
    } else if (
      parameter === 38 &&
      parameters[index + 1] === 5 &&
      validByte(parameters[index + 2])
    ) {
      style.foreground = null;
      style.rgb = xtermColor(parameters[index + 2]);
      index += 2;
    } else if (
      parameter === 38 &&
      parameters[index + 1] === 2 &&
      parameters.slice(index + 2, index + 5).every(validByte)
    ) {
      style.foreground = null;
      style.rgb = parameters.slice(index + 2, index + 5);
      index += 4;
    }
  }
}

function readCsi(input, start) {
  let index = start;
  while (index < input.length) {
    const code = input.charCodeAt(index);
    if (code >= 0x40 && code <= 0x7e) {
      return { end: index + 1, final: input[index], parameters: input.slice(start, index) };
    }
    index += 1;
  }
  return { end: input.length, final: null, parameters: "" };
}

function readOsc(input, start) {
  let index = start;
  while (index < input.length) {
    if (input.charCodeAt(index) === 0x07) return index + 1;
    if (input[index] === "\u001b" && input[index + 1] === "\\") return index + 2;
    index += 1;
  }
  return input.length;
}

export function parseAnsi(input) {
  const lines = [[]];
  const style = defaultStyle();
  let buffer = "";

  const flush = () => {
    if (!buffer) return;
    const line = lines.at(-1);
    const className = styleClass(style);
    const inlineStyle = style.rgb
      ? `--ansi-dark: rgb(${style.rgb.join(", ")}); --ansi-light: rgb(${lightThemeColor(style.rgb).join(", ")})`
      : undefined;
    const previous = line.at(-1);
    if (previous?.className === className && previous?.style === inlineStyle)
      previous.text += buffer;
    else {
      const segment = { text: buffer, className };
      if (inlineStyle) segment.style = inlineStyle;
      line.push(segment);
    }
    buffer = "";
  };

  for (let index = 0; index < input.length; ) {
    const character = input[index];
    const code = input.charCodeAt(index);

    if (character === "\u001b" && input[index + 1] === "[") {
      flush();
      const sequence = readCsi(input, index + 2);
      if (sequence.final === "m") applySgr(style, sequence.parameters);
      index = sequence.end;
      continue;
    }
    if (code === 0x9b) {
      flush();
      const sequence = readCsi(input, index + 1);
      if (sequence.final === "m") applySgr(style, sequence.parameters);
      index = sequence.end;
      continue;
    }
    if (character === "\u001b" && input[index + 1] === "]") {
      flush();
      index = readOsc(input, index + 2);
      continue;
    }
    if (character === "\u001b") {
      flush();
      index = Math.min(index + 2, input.length);
      continue;
    }
    if (character === "\n") {
      flush();
      lines.push([]);
      index += 1;
      continue;
    }
    if (character === "\r") {
      index += 1;
      continue;
    }
    if (character === "\t" || code >= 0x20) buffer += character;
    index += 1;
  }
  flush();
  return lines;
}
