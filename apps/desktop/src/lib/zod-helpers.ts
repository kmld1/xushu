export function hasDisallowedControlCharacter(value: string, allowLineBreaks = false): boolean {
  for (const character of value) {
    const codePoint = character.codePointAt(0);
    if (codePoint === undefined) {
      continue;
    }

    const isControl = codePoint <= 0x1f || codePoint === 0x7f;
    if (!isControl) {
      continue;
    }

    if (allowLineBreaks && (codePoint === 0x09 || codePoint === 0x0a || codePoint === 0x0d)) {
      continue;
    }

    return true;
  }

  return false;
}
