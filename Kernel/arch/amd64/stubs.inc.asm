;
;
;
EXPORT log
EXPORT log2
EXPORT log10
EXPORT pow
EXPORT exp
EXPORT exp2
EXPORT ceil
EXPORT floor
EXPORT fmod
EXPORT round
EXPORT trunc
EXPORT fdim
EXPORT fma
EXPORT sqrt
EXPORT logf
EXPORT log2f
EXPORT log10f
EXPORT powf
EXPORT expf
EXPORT exp2f
EXPORT ceilf
EXPORT floorf
EXPORT fmodf
EXPORT roundf
EXPORT truncf
EXPORT fdimf
EXPORT fmaf
EXPORT sqrtf
; Softmath conversions
EXPORT __fixsfqi	; Single Float -> ? Int
EXPORT __fixsfhi	; Single Float -> ? Int
EXPORT __fixdfqi	; Double Float -> ? Int
EXPORT __fixdfhi
EXPORT __fixunssfqi
EXPORT __fixunssfhi
EXPORT __fixunsdfqi
EXPORT __fixunsdfhi
	jmp halt 

halt:
	cli
	hlt
	jmp halt
