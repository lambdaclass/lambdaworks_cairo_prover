func main() {
    // Call fib(1, 1, 5000).
    let result: felt = fib(1, 1, 5000);

    // Make sure the 5000th Fibonacci number is 529451847554265076639542422726571337094129671408026182306040934618673389596.
    assert result = 529451847554265076639542422726571337094129671408026182306040934618673389596; 
    ret;
}

func fib(first_element, second_element, n) -> (res: felt) {
    jmp fib_body if n != 0;
    tempvar result = second_element;
    return (second_element,);

    fib_body:
    tempvar y = first_element + second_element;
    return fib(second_element, y, n - 1);
}

