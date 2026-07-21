import { describe, expect, it } from "vitest";

import { lightThemeColor, parseAnsi } from "./ansi.js";

describe("parseAnsi", () => {
  it("keeps text and supported SGR foreground colors", () => {
    expect(parseAnsi("plain \u001b[31mred\u001b[1m!\u001b[0m\nnext")).toEqual([
      [
        { text: "plain ", className: "" },
        { text: "red", className: "ansi-red" },
        { text: "!", className: "ansi-red ansi-bold" },
      ],
      [{ text: "next", className: "" }],
    ]);
  });

  it("renders the 256-color and truecolor SGR used by agent panes", () => {
    expect(parseAnsi("\u001b[38;5;244mgray\u001b[38;2;12;34;56mrgb\u001b[39mplain")).toEqual([
      [
        {
          text: "gray",
          className: "ansi-extended",
          style: "--ansi-dark: rgb(128, 128, 128); --ansi-light: rgb(103, 103, 103)",
        },
        {
          text: "rgb",
          className: "ansi-extended",
          style: "--ansi-dark: rgb(12, 34, 56); --ansi-light: rgb(12, 34, 56)",
        },
        { text: "plain", className: "" },
      ],
    ]);
  });

  it("maps bright extended colors to readable Washi ink", () => {
    expect(lightThemeColor([255, 255, 255])).toEqual([108, 108, 108]);
    expect(lightThemeColor([175, 215, 255])).toEqual([90, 112, 134]);
    expect(lightThemeColor([255, 215, 0])).toEqual([120, 100, 0]);
  });

  it("drops OSC payloads and non-SGR CSI control sequences", () => {
    const value = "safe\u001b]0;<img src=x onerror=alert(1)>\u0007\u001b[2J\u001b[Htext";
    expect(parseAnsi(value)).toEqual([[{ text: "safetext", className: "" }]]);
  });

  it("drops unterminated control strings and C0 controls", () => {
    expect(parseAnsi("ok\u0000\u0008!\u001b]8;;https://example.test")).toEqual([
      [{ text: "ok!", className: "" }],
    ]);
  });

  it("returns text as data without generating HTML", () => {
    expect(parseAnsi("<script>alert(1)</script>")).toEqual([
      [{ text: "<script>alert(1)</script>", className: "" }],
    ]);
  });
});
