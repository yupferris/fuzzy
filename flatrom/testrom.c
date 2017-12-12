// quick, easy types            Byte(s)           Range
typedef unsigned char   u8;     /* 1               0 ...           255 */
typedef unsigned short  u16;    /* 2               0 ...        65,535 */
typedef unsigned long   u32;    /* 4               0 ... 4,294,967,295 */

typedef signed char     s8;     /* 1            -128 ...           127*/
typedef signed short    s16;    /* 2         -32,768 ...        32,767*/
typedef signed long     s32;    /* 4  -2,147,483,648 ... 2,147,483,647 */

typedef unsigned char   BYTE;   /* 1               0 ...           255 */
typedef unsigned short  HWORD;  /* 2               0 ...        65,535 */
typedef unsigned long   WORD;   /* 4               0 ... 4,294,967,295 */

#define     BGMMBase         0x00020000                 // Base address of BGMap Memory
#define     BGMap(b)         (BGMMBase + (b * 0x2000))  // Address of BGMap b (0 <= b <= 13)

// Print

#define PRINT_ROWS 28
#define PRINT_COLS 48

static u16 *printMapPos;
static int printColumn;
static int printRow;

void printInit()
{
    printMapPos = (u16 *)BGMap(0);
    printColumn = printRow = 0;
}

void printStr(const char *str)
{
    while (1)
    {
        char c = *str++;

        if (!c)
            break;

        if (c == '\n' || printColumn == PRINT_COLS)
        {
            printMapPos -= printColumn;
            printColumn = 0;

            printRow++;
            if (printRow >= PRINT_ROWS)
            {
                // Once we're offscreen, wrap around to make sure this stays fast
                printInit();
            }
            else
            {
                printMapPos += 64;
            }
        }
        else
        {
            *printMapPos++ = c;
            printColumn++;
        }
    }
}

static const char digits[16] = { '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f' };

void printU32(const u32 value)
{
    char buf[9];
    int i;

    //printStr("0x");

    buf[8] = '\0';

    for (i = 0; i < 8; i++)
        buf[i] = digits[(value >> (4 * (7 - i))) & 0x0f];

    printStr(buf);
}

void printClear()
{
    int x, y;
    u16 *mapPtr = (u16 *)BGMap(0);

    for (y = 0; y < 64; y++)
    {
        for (x = 0; x < 64; x++)
            *(mapPtr++) = 0x0000;
    }

    printInit();
}

int main()
{
    // Set up printing & clear bg map data
    printInit();
    printClear();

    // Let's goooo!
    printStr("hello from C!!!!!11\n");

    return 42;
}
