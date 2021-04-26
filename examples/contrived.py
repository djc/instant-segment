import instant_segment


def main():
    unigrams = []
    unigrams.append(("choose", 50))
    unigrams.append(("chooses", 10))
    unigrams.append(("spain", 50))
    unigrams.append(("pain", 10))

    bigrams = []
    bigrams.append((("choose", "spain"), 10))
    bigrams.append((("chooses", "pain"), 10))

    segmenter = instant_segment.Segmenter(iter(unigrams), iter(bigrams))
    search = instant_segment.Search()
    segmenter.segment("choosespain", search)
    print([word for word in search])


if __name__ == "__main__":
    main()
