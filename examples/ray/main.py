import ray


@ray.remote
def work(x):
    print(f"Hello, I am working on doubling {x}!")
    return x * 2


def main():
    print("Hello, I am the submitter!")

    with ray.init():
        futures = [work.remote(i) for i in range(2)]
        print(ray.get(futures))


if __name__ == "__main__":
    main()
