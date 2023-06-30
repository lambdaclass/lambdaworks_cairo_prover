func conditional(x) -> felt {
    if (x == 5) {
        return x + 1;
    } else {
        return x * 2;
    }
}

func main() {
    let x = 5;
    let y = conditional(x);
    return ();
}
