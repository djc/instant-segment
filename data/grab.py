import urllib.request
import gzip
import shutil
import os.path

BLOCK_SIZE = 4 * 1024 * 1024

UNIGRAM_PARTS = 24
BIGRAM_PARTS = 589
NGRAM_URL = 'http://storage.googleapis.com/books/ngrams/books/20200217/eng/{n}-{part:05}-of-{parts:05}.gz'

WORD_LIST_URL = 'http://app.aspell.net/create?max_size=60&spelling=US&spelling=GBs&spelling=GBz&spelling=CA&spelling=AU&max_variant=2&diacritic=strip&special=hacker&download=wordlist&encoding=utf-8&format=inline'


def download(url, fn):
    '''Download the given url and write it to the given fn

    Download in blocks of size BLOCK_SIZE and report progress after every 5% of the file.
    '''
    try:
        u = urllib.request.urlopen(url)
    except urllib.error.HTTPError as err:
        print(f'error for {url}: {err}')
        return 0

    with open(fn + '.tmp', 'wb') as f:
        length = u.info().get('Content-Length')
        print('downloading: %s (%s bytes)' % (fn, length))

        downloaded, complete, notified = 0, 0.0, 0.0
        while True:
            buf = u.read(BLOCK_SIZE)
            if not buf:
                break

            downloaded += len(buf)
            f.write(buf)

            if length is None:
                continue

            complete = downloaded / int(length)
            if complete < notified + 0.05:
                continue

            notified = complete
            status = '%10d  [%3.2f%%]' % (downloaded, complete * 100)
            status = status + chr(8) * (len(status) + 1)
            print(status)

    os.rename(fn + '.tmp', fn)
    return complete


def cache(n, part, parts):
    '''Downloads and decompresses ngram data files as necessary

    First downloads, then decompresses the given ngram part file. Will do nothing
    if the decompressed file already exist and will only decompress if the compressed
    file for this part already exists in the proper location.'''
    compressed = f'cache/eng-{n}-{part:05}-{parts:05}.gz'
    plain = compressed[:-3] + '.txt'

    if os.path.isfile(plain):
        return
    elif not os.path.isfile(compressed):
        url = NGRAM_URL.format(n=n, part=part, parts=parts)
        complete = download(url, compressed)
        print(f'downloaded {compressed} ({complete * 100:3.2f}% complete)')

    if os.path.isfile(compressed):
        with open(plain + '.tmp', 'wb') as output, gzip.open(compressed, 'rb') as input:
            output.write(input.read())
        os.rename(plain + '.tmp', plain)
        print(f'decompressed {compressed}')
    else:
        print(f'{compressed} not found')


def main():
    if not os.path.exists('cache'):
        os.mkdir('cache')

    wl_fn = 'cache/eng-wordlist.txt'
    if not os.path.isfile(wl_fn):
        download(WORD_LIST_URL, wl_fn)
    for part in range(UNIGRAM_PARTS):
        cache(1, part, UNIGRAM_PARTS)
    for part in range(BIGRAM_PARTS):
        cache(2, part, BIGRAM_PARTS)

if __name__ == '__main__':
    main()
