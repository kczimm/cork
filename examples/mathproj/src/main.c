#include <stdio.h>
#include "add.h"
#include "../include/subtract.h"

int main() {
    int a = 10, b = 5;
    printf("Add: %d + %d = %d\n", a, b, add(a, b));
    printf("Subtract: %d - %d = %d\n", a, b, subtract(a, b));
    return 0;
}
