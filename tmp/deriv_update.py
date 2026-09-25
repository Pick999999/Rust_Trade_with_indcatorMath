import re

with open('src/deriv.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Just verifying we can read it.
print("Length of original:", len(content))
