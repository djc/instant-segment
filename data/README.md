# Example data files

The data files in this directory are derived from the Google Books Ngram data
([version 3, 2020-02-17][ngrams]) and the [SCOWL][scowl] word list.

## Recreating these files

The code used to build the provided `en-unigrams.txt` and `en-bigrams.txt`
is provided as part of this repository. To recreate the files, first
download the input data. Note that this will download about 300 GB of
data and unpack it to about 1.4 TB of data.

```
instant-segment $ cd data
data $ python3 grab.py
```

After the data has been downloaded, run the `merge` tool to create the word lists:

```
instant-segment $ cargo run --release --example merge
```

## License

The SCOWL word list is licensed under a number of licenses detailed in
[LICENSE-SCOWL](./LICENSE-SCOWL), which the website describes as MIT-like.

The Ngram data provided by Google is available under the Creative Commons
Attribution 3.0 Unported license:

> This work is licensed under the Creative Commons Attribution 3.0 Unported
> License. To view a copy of this license, visit
> http://creativecommons.org/licenses/by/3.0/ or send a letter to Creative
> Commons, PO Box 1866, Mountain View, CA 94042, USA.

[ngrams]: https://storage.googleapis.com/books/ngrams/books/datasetsv3.html
[scowl]: http://wordlist.aspell.net/
