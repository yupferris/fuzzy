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

/***** Display RAM *****/
//extern u32* const   L_FRAME0;
#define     CharSeg0         0x00006000                 // Characters 0-511
//extern u32* const   L_FRAME1;
#define     CharSeg1         0x0000E000                 // Characters 512-1023
//extern u32* const   R_FRAME0;
#define     CharSeg2         0x00016000                 // Characters 1024-1535
//extern u32* const   R_FRAME1;
#define     CharSeg3         0x0001E000                 // Characters 1536-2047
#define     BGMMBase         0x00020000                 // Base address of BGMap Memory
//extern u16* const   BGMM;
#define     BGMap(b)         (BGMMBase + (b * 0x2000))  // Address of BGMap b (0 <= b <= 13)

#define     WAMBase          0x0003D800                 // Base address of World Attribute Memory
//extern u16* const   WAM;
#define     World(w)         (WAMBase + (w * 0x0020))   // Address of World w (0 <= w <= 31)
//extern u16* const   CLMN_TBL;
#define     OAMBase          0x0003E000                 // Base address of Object Attribute Memory
//extern u16* const   OAM;
#define     Object(o)        (OAMBase + (o * 0x0008))   // Address of Obj o (0 <= o <= 1023)

u32* const  L_FRAME0 =  (u32*)0x00000000;   // Left Frame Buffer 0
u32* const  L_FRAME1 =  (u32*)0x00008000;   // Left Frame Buffer 1
u32* const  R_FRAME0 =  (u32*)0x00010000;   // Right Frame Buffer 0
u32* const  R_FRAME1 =  (u32*)0x00018000;   // Right Frame Buffer 1
u16* const  BGMM =      (u16*)BGMMBase;     // Pointer to BGMM
u16* const  WAM =       (u16*)WAMBase;      // Pointer to WAM
u16* const  CLMN_TBL =  (u16*)0x0003DC00;   // Base address of Column Tables
u16* const  OAM =       (u16*)OAMBase;      // Pointer to OAM

/* Macro to set the brightness registers */
#define SET_BRIGHT(a,b,c)       VIP_REGS[BRTA]=(u16)(a);  \
                                VIP_REGS[BRTB]=(u16)(b);  \
                                VIP_REGS[BRTC]=(u16)(c)

/* Macro to set the GPLT (BGMap palette) */
#define SET_GPLT(n,pal)         VIP_REGS[GPLT0+n]=pal

/* Macro to set the JPLT (OBJ palette) */
#define SET_JPLT(n,pal)         VIP_REGS[JPLT0+n]=pal

/* Defines for INTPND\INTENB\INTCLR */
#define TIMEERR     0x8000
#define XPEND       0x4000
#define SBHIT       0x2000
#define FRAMESTART  0x0010
#define GAMESTART   0x0008
#define RFBEND      0x0004
#define LFBEND      0x0002
#define SCANERR     0x0001

/* Defines for DPSTTS\DPCTRL */
#define LOCK        0x0400  // VPU SELECT CTA
#define SYNCE       0x0200  // L,R_SYNC TO VPU
#define RE      0x0100  // MEMORY REFLASH CYCLE ON
#define FCLK        0x0080
#define SCANRDY     0x0040
#define DISP        0x0002  // DISPLAY ON
#define DPRST       0x0001  // RESET VPU COUNTER AND WAIT FCLK

/* Defines for XPSTTS\XPCTRL */
#define SBOUT       0x8000  // In FrameBuffer drawing included
#define OVERTIME    0x0010  // Processing
#define XPBSYR      0x000C  // In the midst of drawing processing reset
#define XPBSY1      0x0008  // In the midst of FrameBuffer1 picture editing
#define XPBSY0      0x0004  // In the midst of FrameBuffer0 picture editing
#define XPEN        0x0002  // Start of drawing
#define XPRST       0x0001  // Forcing idling


/****** VIP Registers ******/
volatile u16* VIP_REGS = (u16*)0x0005F800;

/****** VIP Register Mnemonics ******/
#define INTPND  0x00
#define INTENB  0x01
#define INTCLR  0x02

#define DPSTTS  0x10
#define DPCTRL  0x11
#define BRTA    0x12
#define BRTB    0x13
#define BRTC    0x14
#define REST    0x15

#define FRMCYC  0x17
#define CTA 0x18

#define XPSTTS  0x20
#define XPCTRL  0x21
#define VER 0x22

#define SPT0    0x24
#define SPT1    0x25
#define SPT2    0x26
#define SPT3    0x27

#define GPLT0   0x30
#define GPLT1   0x31
#define GPLT2   0x32
#define GPLT3   0x33

#define JPLT0   0x34
#define JPLT1   0x35
#define JPLT2   0x36
#define JPLT3   0x37

#define BKCOL   0x38

typedef struct WORLD 
{
    u16 head;
    u16 gx;
    s16 gp;
    u16 gy;
    u16 mx;
    s16 mp;
    u16 my;
    u16 w;
    u16 h;
    u16 param;
    u16 ovr;
    u16 spacer[5];
} WORLD;

WORLD* const WA = (WORLD*)0x0003D800;

/* "vbSetWorld" header flags */
/* (OR these together to build a World Header) */

#define WRLD_ON     0xC000  // There_are_two_screens!__USE_THEM!!!
#define WRLD_LON    0x8000
#define WRLD_RON    0x4000
#define WRLD_OBJ    0x3000
#define WRLD_AFFINE 0x2000
#define WRLD_HBIAS  0x1000
#define WRLD_BGMAP  0x0000

#define WRLD_1x1    0x0000
#define WRLD_1x2    0x0100
#define WRLD_1x4    0x0200
#define WRLD_1x8    0x0300
#define WRLD_2x1    0x0400
#define WRLD_2x2    0x0500
#define WRLD_2x4    0x0600
#define WRLD_4x2    0x0900
#define WRLD_4x1    0x0800
#define WRLD_8x1    0x0C00

#define WRLD_OVR    0x0080
#define WRLD_END    0x0040

/* Macros for world manipulation */
// (Obsoleted by the WA array of WORLD structures...)
#define WORLD_HEAD(n,head)      WAM[(n << 4)] = head
#define WORLD_GSET(n,gx,gp,gy)  WAM[(n << 4) + 1] = gx; WAM[(n << 4) + 2] = gp; WAM[(n << 4) + 3] = gy
#define WORLD_MSET(n,mx,mp,my)  WAM[(n << 4) + 4] = mx; WAM[(n << 4) + 5] = mp; WAM[(n << 4) + 6] = my
#define WORLD_SIZE(n,w,h)       WAM[(n << 4) + 7] = w; WAM[(n << 4) + 8] = h
#define WORLD_PARAM(n,p)        WAM[(n << 4) + 9] = ((p - 0x20000) >> 1) & 0xFFF0
#define WORLD_OVER(n,o)         WAM[(n << 4) + 10] = o

u8 const colTable[128] = 
{
    0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE,
    0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE,
    0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE,
    0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE,
    0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE,
    0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE,
    0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE,
    0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xE0, 0xBC,
    0xA6, 0x96, 0x8A, 0x82, 0x7A, 0x74, 0x6E, 0x6A,
    0x66, 0x62, 0x60, 0x5C, 0x5A, 0x58, 0x56, 0x54,
    0x52, 0x50, 0x50, 0x4E, 0x4C, 0x4C, 0x4A, 0x4A,
    0x48, 0x48, 0x46, 0x46, 0x46, 0x44, 0x44, 0x44,
    0x42, 0x42, 0x42, 0x40, 0x40, 0x40, 0x40, 0x40,
    0x3E, 0x3E, 0x3E, 0x3E, 0x3E, 0x3E, 0x3E, 0x3C,
    0x3C, 0x3C, 0x3C, 0x3C, 0x3C, 0x3C, 0x3C, 0x3C,
    0x3C, 0x3C, 0x3C, 0x3C, 0x3C, 0x3C, 0x3C, 0x3C
};

// Setup the default Column Table
void vbSetColTable() 
{
    u8 i;

    for (i = 0; i <= 127; i++) {
        CLMN_TBL[i      ] = colTable[i];
        CLMN_TBL[i + 256] = colTable[i];
        CLMN_TBL[i + 128] = colTable[127 - i];
        CLMN_TBL[i + 384] = colTable[127 - i];
    }
}