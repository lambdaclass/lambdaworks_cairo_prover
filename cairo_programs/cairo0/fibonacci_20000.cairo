func main() {
    // Call fib(1, 1, 20000).
    let result: felt = fib(1, 1, 20000);

    // Make sure the 20000th Fibonacci number is 2466807170979865163394825981097502788813037637400933247258687309819819983611.
    assert result = 2466807170979865163394825981097502788813037637400933247258687309819819983611; 
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

