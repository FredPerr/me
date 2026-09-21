import { PATH_TRUNCATE_LENGTH } from './config.ts';

const ELLIPSIS = '…';
const JOINER = `${ELLIPSIS}/`;
const TRAILING_SLASHES = /\/+$/;

export function truncateMiddle(filePath: string, maxLength = PATH_TRUNCATE_LENGTH): string {
  if (filePath.length <= maxLength) return filePath;

  const fileName = extractFileName(filePath);
  if (fileNameFillsWidth(fileName, maxLength)) {
    return ELLIPSIS + tailOf(fileName, maxLength - ELLIPSIS.length);
  }
  return joinHeadAndFile(filePath, fileName, maxLength);
}

function extractFileName(filePath: string): string {
  return filePath.slice(filePath.lastIndexOf('/') + 1);
}

function fileNameFillsWidth(fileName: string, maxLength: number): boolean {
  return fileName.length + ELLIPSIS.length >= maxLength;
}

function tailOf(text: string, length: number): string {
  return text.slice(text.length - length);
}

function joinHeadAndFile(filePath: string, fileName: string, maxLength: number): string {
  const headLength = maxLength - fileName.length - JOINER.length;
  const head = filePath.slice(0, headLength).replace(TRAILING_SLASHES, '');
  return `${head}${JOINER}${fileName}`;
}
