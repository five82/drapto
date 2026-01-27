package drapto

import (
	"testing"
)

func TestParseCRF(t *testing.T) {
	tests := []struct {
		name    string
		input   string
		wantSD  uint8
		wantHD  uint8
		wantUHD uint8
		wantErr bool
	}{
		{
			name:    "single value",
			input:   "27",
			wantSD:  27,
			wantHD:  27,
			wantUHD: 27,
		},
		{
			name:    "single value with whitespace",
			input:   "  27  ",
			wantSD:  27,
			wantHD:  27,
			wantUHD: 27,
		},
		{
			name:    "triple value",
			input:   "25,27,29",
			wantSD:  25,
			wantHD:  27,
			wantUHD: 29,
		},
		{
			name:    "triple value with whitespace",
			input:   " 25 , 27 , 29 ",
			wantSD:  25,
			wantHD:  27,
			wantUHD: 29,
		},
		{
			name:    "zero CRF is valid",
			input:   "0",
			wantSD:  0,
			wantHD:  0,
			wantUHD: 0,
		},
		{
			name:    "max CRF is valid",
			input:   "63",
			wantSD:  63,
			wantHD:  63,
			wantUHD: 63,
		},
		{
			name:    "empty string",
			input:   "",
			wantErr: true,
		},
		{
			name:    "whitespace only",
			input:   "   ",
			wantErr: true,
		},
		{
			name:    "CRF over max",
			input:   "64",
			wantErr: true,
		},
		{
			name:    "negative value",
			input:   "-1",
			wantErr: true,
		},
		{
			name:    "non-numeric",
			input:   "abc",
			wantErr: true,
		},
		{
			name:    "two values",
			input:   "25,27",
			wantErr: true,
		},
		{
			name:    "four values",
			input:   "25,27,29,31",
			wantErr: true,
		},
		{
			name:    "invalid SD in triple",
			input:   "abc,27,29",
			wantErr: true,
		},
		{
			name:    "invalid HD in triple",
			input:   "25,abc,29",
			wantErr: true,
		},
		{
			name:    "invalid UHD in triple",
			input:   "25,27,abc",
			wantErr: true,
		},
		{
			name:    "SD over max in triple",
			input:   "64,27,29",
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			sd, hd, uhd, err := ParseCRF(tt.input)
			if (err != nil) != tt.wantErr {
				t.Errorf("ParseCRF(%q) error = %v, wantErr %v", tt.input, err, tt.wantErr)
				return
			}
			if tt.wantErr {
				return
			}
			if sd != tt.wantSD {
				t.Errorf("ParseCRF(%q) SD = %d, want %d", tt.input, sd, tt.wantSD)
			}
			if hd != tt.wantHD {
				t.Errorf("ParseCRF(%q) HD = %d, want %d", tt.input, hd, tt.wantHD)
			}
			if uhd != tt.wantUHD {
				t.Errorf("ParseCRF(%q) UHD = %d, want %d", tt.input, uhd, tt.wantUHD)
			}
		})
	}
}
