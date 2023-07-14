func main() {
    // Call fib(1, 1, 2000).
    let result: felt = fib(1, 1, 2000);

    // Make sure the 2000th Fibonacci number is 2488789449880543025993160600549605444069072274860741457936217360575764513035.
    assert result = 2488789449880543025993160600549605444069072274860741457936217360575764513035; 
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

