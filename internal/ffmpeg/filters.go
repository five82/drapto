package ffmpeg

import "strings"

// VideoFilterChain builds video filter chains.
type VideoFilterChain struct {
	filters []string
}

// NewVideoFilterChain creates a new empty filter chain.
func NewVideoFilterChain() *VideoFilterChain {
	return &VideoFilterChain{}
}

// AddCrop adds a crop filter to the chain.
func (c *VideoFilterChain) AddCrop(crop string) *VideoFilterChain {
	if crop != "" {
		c.filters = append(c.filters, crop)
	}
	return c
}

// AddFilter adds a custom filter to the chain.
func (c *VideoFilterChain) AddFilter(filter string) *VideoFilterChain {
	if filter != "" {
		c.filters = append(c.filters, filter)
	}
	return c
}

// Build builds the filter chain into a single filter string.
// Returns empty string if no filters are present.
func (c *VideoFilterChain) Build() string {
	if len(c.filters) == 0 {
		return ""
	}
	return strings.Join(c.filters, ",")
}

// IsEmpty returns true if no filters are present.
func (c *VideoFilterChain) IsEmpty() bool {
	return len(c.filters) == 0
}
