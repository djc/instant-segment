import instant_segment


def main():
    unigrams = []
    unigrams.append(("choose", 80_000))
    unigrams.append(("chooses", 7_000))
    unigrams.append(("spain", 20_000))
    unigrams.append(("pain", 90_000))

    bigrams = []
    bigrams.append((("choose", "spain"), 7))
    bigrams.append((("chooses", "pain"), 0))

    segmenter = instant_segment.Segmenter(iter(unigrams), iter(bigrams))
    search = instant_segment.Search()
    segmenter.segment("choosespain", search)
    print([word for word in search])


if __name__ == "__main__":
    main()
