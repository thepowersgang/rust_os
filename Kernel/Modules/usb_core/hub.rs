
#[repr(C)]
#[derive(Debug)]
pub enum PortFeature
{
	Connection,
	Enable,
	Suspend,
	OverCurrent,
	Reset,
	Power,
	LowSpeed,
	CConnection = 16,
	CEnable,
	CSuspend,
	COverCurrent,
	CReset,
	Test,
	Indicator,
}

