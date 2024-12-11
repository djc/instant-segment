import instant_segment, os, sys

DATA_DIR = os.path.join(os.path.dirname(__file__), '../../data/')

def unigrams():
    for ln in open(os.path.join(DATA_DIR, 'en-unigrams.txt')):
        parts = ln.split('\t', 1)
        yield (parts[0], float(parts[1].strip()))

def bigrams():
    for ln in open(os.path.join(DATA_DIR, 'en-bigrams.txt')):
        word_split = ln.split(' ', 1)
        score_split = word_split[1].split('\t', 1)
        yield ((word_split[0], score_split[0]), float(score_split[1].strip()))

def main():
    segmenter = instant_segment.Segmenter(unigrams(), bigrams())
    search = instant_segment.Search()
    score = segmenter.segment('thisisatest', search)
    print(f"{score=}")
    print([word for word in search])

if __name__ == '__main__':
    main()
